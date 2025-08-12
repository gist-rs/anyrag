//! # Knowledge Base Ingestion Pipeline
//!
//! This module implements the 5-stage virtuous cycle for building a knowledge base:
//! 1.  **Ingestion & Caching**: Fetches raw web content and stores it, avoiding reprocessing.
//! 2.  **Distillation & Augmentation**: Uses an LLM to extract explicit FAQs and generate new ones.
//! 3.  **Structured Storage**: Stores the structured data in SQLite for retrieval.
//! 4.  **Hybrid Retrieval**: The RAG query process (implemented elsewhere).
//! 5.  **Fine-Tuning Export**: Exports the knowledge base into a format for model fine-tuning.

use crate::{
    errors::PromptError,
    prompts::knowledge::{
        AUGMENTATION_SYSTEM_PROMPT, AUGMENTATION_USER_PROMPT, KNOWLEDGE_EXTRACTION_SYSTEM_PROMPT,
        KNOWLEDGE_EXTRACTION_USER_PROMPT,
    },
    providers::ai::AiProvider,
};
use futures::stream::{self, StreamExt};
use md5;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};
use turso::{params, Connection, Database};

// --- Error Definitions ---

#[derive(Error, Debug)]
pub enum KnowledgeError {
    #[error("Database error: {0}")]
    Database(#[from] turso::Error),
    #[error("Failed to fetch content: {0}")]
    Fetch(#[from] reqwest::Error),
    #[error("LLM processing failed: {0}")]
    Llm(#[from] PromptError),
    #[error("Failed to parse LLM response: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("Content has not changed for URL: {0}, skipping.")]
    ContentUnchanged(String),
    #[error("Jina Reader API request failed with status {status}: {body}")]
    JinaReaderFailed { status: u16, body: String },
    #[error("Failed to convert database value: expected text, found other type.")]
    TypeConversion,
}

// --- Data Structures ---

/// Represents the raw content fetched from a URL before LLM processing.
#[derive(Debug, Clone)]
pub struct RawContent {
    pub url: String,
    pub markdown_content: String,
    pub content_hash: String,
}

/// Represents the structured data extracted by the LLM in the first pass.
#[derive(Serialize, Deserialize, Debug)]
pub struct ExtractedKnowledge {
    #[serde(default)]
    pub faqs: Vec<FaqItem>,
    #[serde(default)]
    pub content_chunks: Vec<ContentChunk>,
}

/// Represents an explicit or generated FAQ.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FaqItem {
    pub question: String,
    pub answer: String,
    pub is_explicit: bool,
}

/// Represents a chunk of informational content to be augmented into an FAQ.
#[derive(Serialize, Deserialize, Debug)]
pub struct ContentChunk {
    pub topic: String,
    pub content: String,
}

/// Represents the JSON structure for the augmentation LLM call's response.
#[derive(Deserialize, Debug)]
struct AugmentationResponse {
    question: String,
}

// --- Pipeline Orchestration ---

/// Orchestrates the full ingestion pipeline (Stages 1-3) for a given URL.
pub async fn run_ingestion_pipeline(
    db: &Database,
    ai_provider: &dyn AiProvider,
    url: &str,
) -> Result<usize, KnowledgeError> {
    // Stage 1: Ingest and Cache
    let raw_content = match ingest_and_cache_url(db, url).await {
        Ok(content) => content,
        Err(KnowledgeError::ContentUnchanged(url)) => {
            info!("Content for {} is unchanged, pipeline finished.", url);
            return Ok(0);
        }
        Err(e) => return Err(e),
    };

    // Stage 2: Distill and Augment
    let faq_items = distill_and_augment(ai_provider, &raw_content).await?;

    // Stage 3: Store Structured Knowledge
    store_structured_knowledge(db, &raw_content.url, &raw_content.content_hash, faq_items).await
}

// --- Stage 1: Ingestion & Caching ---

/// Ensures the necessary tables for the knowledge base pipeline exist.
pub async fn create_kb_tables_if_not_exists(conn: &Connection) -> Result<(), turso::Error> {
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS raw_content (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            url TEXT UNIQUE NOT NULL,
            markdown_content TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            last_fetched TEXT NOT NULL
        );
        "#,
        (),
    )
    .await?;

    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS faq_kb (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            question TEXT NOT NULL,
            answer TEXT NOT NULL,
            source_url TEXT NOT NULL,
            is_explicit BOOLEAN NOT NULL,
            content_hash TEXT NOT NULL,
            last_modified TEXT NOT NULL,
            embedding BLOB
        );
        "#,
        (),
    )
    .await?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_faq_kb_source_url ON faq_kb(source_url);",
        (),
    )
    .await?;

    Ok(())
}

/// Fetches clean Markdown from a URL using the Jina Reader service.
async fn fetch_markdown_from_url(url: &str) -> Result<String, KnowledgeError> {
    let jina_url = format!("https://r.jina.ai/{url}");
    info!("Fetching clean markdown from: {jina_url}");
    let response = reqwest::get(&jina_url)
        .await
        .map_err(KnowledgeError::Fetch)?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(KnowledgeError::JinaReaderFailed { status, body });
    }

    response.text().await.map_err(KnowledgeError::Fetch)
}

/// **Stage 1**: Fetches content, checks if it's new, and stores it in `raw_content`.
async fn ingest_and_cache_url(db: &Database, url: &str) -> Result<RawContent, KnowledgeError> {
    let conn = db.connect()?;
    create_kb_tables_if_not_exists(&conn).await?;

    let markdown_content = fetch_markdown_from_url(url).await?;
    let content_hash = format!("{:x}", md5::compute(markdown_content.as_bytes()));
    let now = chrono::Utc::now().to_rfc3339();

    if let Some(row) = conn
        .query(
            "SELECT content_hash FROM raw_content WHERE url = ?",
            params![url],
        )
        .await?
        .next()
        .await?
    {
        if let Ok(existing_hash) = row.get_value(0) {
            let existing_hash_str = match existing_hash {
                turso::Value::Text(s) => s,
                _ => return Err(KnowledgeError::TypeConversion),
            };
            if existing_hash_str == content_hash {
                return Err(KnowledgeError::ContentUnchanged(url.to_string()));
            }
        }
    }

    // First, delete any existing record for this URL. Then, insert the new one.
    // This two-step process is a portable equivalent of the unsupported `REPLACE INTO`.
    conn.execute("DELETE FROM raw_content WHERE url = ?", params![url])
        .await?;
    conn.execute(
        "INSERT INTO raw_content (url, markdown_content, content_hash, last_fetched) VALUES (?, ?, ?, ?)",
        params![url, markdown_content.clone(), content_hash.clone(), now],
    ).await?;

    info!("Successfully stored new/updated raw content for URL: {url}");
    Ok(RawContent {
        url: url.to_string(),
        markdown_content,
        content_hash,
    })
}

// --- Stage 2: LLM-Powered Distillation & Augmentation ---

/// **Stage 2**: Takes raw content, extracts explicit FAQs, and generates new
/// ones from informational chunks using a two-pass LLM process.
pub async fn distill_and_augment(
    ai_provider: &dyn AiProvider,
    raw_content: &RawContent,
) -> Result<Vec<FaqItem>, KnowledgeError> {
    info!("Starting Pass 1: Extraction for URL: {}", raw_content.url);
    let user_prompt = KNOWLEDGE_EXTRACTION_USER_PROMPT
        .replace("{markdown_content}", &raw_content.markdown_content);
    let llm_response = ai_provider
        .generate(KNOWLEDGE_EXTRACTION_SYSTEM_PROMPT, &user_prompt)
        .await?;
    debug!("LLM extraction response: {}", llm_response);
    let mut extracted_data: ExtractedKnowledge = serde_json::from_str(&llm_response)?;
    info!(
        "Pass 1 complete. Found {} explicit FAQs and {} content chunks.",
        extracted_data.faqs.len(),
        extracted_data.content_chunks.len()
    );

    if !extracted_data.content_chunks.is_empty() {
        info!(
            "Starting Pass 2: Augmentation for {} content chunks.",
            extracted_data.content_chunks.len()
        );
        let augmented_faqs: Vec<FaqItem> = stream::iter(extracted_data.content_chunks)
            .map(|chunk| {
                async move {
                    let user_prompt =
                        AUGMENTATION_USER_PROMPT.replace("{content_chunk}", &chunk.content);
                    match ai_provider.generate(AUGMENTATION_SYSTEM_PROMPT, &user_prompt).await {
                        Ok(resp) => match serde_json::from_str::<AugmentationResponse>(&resp) {
                            Ok(parsed) => Some(FaqItem {
                                question: parsed.question,
                                answer: chunk.content,
                                is_explicit: false,
                            }),
                            Err(e) => {
                                warn!("Failed to parse augmentation response for chunk '{}': {}. Skipping.", chunk.topic, e);
                                None
                            }
                        },
                        Err(e) => {
                            warn!("LLM generation failed for augmentation: {}", e);
                            None
                        }
                    }
                }
            })
            .buffer_unordered(10) // Concurrently process up to 10 chunks
            .filter_map(|x| async move { x })
            .collect()
            .await;
        info!(
            "Pass 2 complete. Generated {} new FAQs.",
            augmented_faqs.len()
        );
        extracted_data.faqs.extend(augmented_faqs);
    }

    Ok(extracted_data.faqs)
}

// --- Stage 3: Structured Storage ---

/// **Stage 3**: Stores the final list of FAQ items into the `faq_kb` table.
pub async fn store_structured_knowledge(
    db: &Database,
    url: &str,
    content_hash: &str,
    faq_items: Vec<FaqItem>,
) -> Result<usize, KnowledgeError> {
    if faq_items.is_empty() {
        info!("No structured knowledge to store for URL: {url}");
        return Ok(0);
    }
    let conn = db.connect()?;
    let now = chrono::Utc::now().to_rfc3339();
    info!("Storing {} FAQ items for URL: {}", faq_items.len(), url);

    conn.execute("DELETE FROM faq_kb WHERE source_url = ?", params![url])
        .await?;
    conn.execute("BEGIN TRANSACTION", ()).await?;
    let mut stmt = conn
        .prepare(
            r#"INSERT INTO faq_kb (question, answer, source_url, is_explicit, content_hash, last_modified) VALUES (?, ?, ?, ?, ?, ?)"#,
        )
        .await?;
    for faq in &faq_items {
        stmt.execute(params![
            faq.question.clone(),
            faq.answer.clone(),
            url,
            faq.is_explicit,
            content_hash,
            now.clone()
        ])
        .await?;
    }
    conn.execute("COMMIT", ()).await?;
    let changes = faq_items.len();
    info!("Successfully stored {changes} new FAQs for URL: {url}");
    Ok(changes)
}

// --- Stage 5: Fine-Tuning Export ---

#[derive(Serialize, Debug)]
struct FinetuningEntry<'a> {
    messages: Vec<FinetuningMessage<'a>>,
}

#[derive(Serialize, Debug)]
struct FinetuningMessage<'a> {
    role: &'a str,
    content: &'a str,
}

/// **Stage 5**: Exports the `faq_kb` table into a JSONL string for fine-tuning.
pub async fn export_for_finetuning(db: &Database) -> Result<String, KnowledgeError> {
    info!("Exporting knowledge base for fine-tuning.");
    let conn = db.connect()?;
    let mut stmt = conn.prepare("SELECT question, answer FROM faq_kb").await?;
    let mut rows = stmt.query(()).await?;
    let system_prompt = "You are a helpful assistant. Provide clear, accurate answers based on the retrieved context.";
    let mut jsonl_output = String::new();

    while let Some(row) = rows.next().await? {
        let question = match row.get_value(0)? {
            turso::Value::Text(s) => s,
            _ => return Err(KnowledgeError::TypeConversion),
        };
        let answer = match row.get_value(1)? {
            turso::Value::Text(s) => s,
            _ => return Err(KnowledgeError::TypeConversion),
        };
        let entry = FinetuningEntry {
            messages: vec![
                FinetuningMessage {
                    role: "system",
                    content: system_prompt,
                },
                FinetuningMessage {
                    role: "user",
                    content: &question,
                },
                FinetuningMessage {
                    role: "assistant",
                    content: &answer,
                },
            ],
        };
        let line = serde_json::to_string(&entry)?;
        jsonl_output.push_str(&line);
        jsonl_output.push('\n');
    }
    info!(
        "Generated fine-tuning data with {} entries.",
        jsonl_output.lines().count()
    );
    Ok(jsonl_output)
}
