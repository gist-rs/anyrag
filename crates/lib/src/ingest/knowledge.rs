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
        AUGMENTATION_SYSTEM_PROMPT, KNOWLEDGE_EXTRACTION_SYSTEM_PROMPT,
        KNOWLEDGE_EXTRACTION_USER_PROMPT,
    },
    providers::ai::AiProvider,
};
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

#[derive(Debug, Clone)]
pub struct RawContent {
    pub url: String,
    pub markdown_content: String,
    pub content_hash: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExtractedKnowledge {
    #[serde(default)]
    faqs: Vec<FaqItem>,
    #[serde(default)]
    content_chunks: Vec<ContentChunk>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FaqItem {
    question: String,
    answer: String,
    is_explicit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ContentChunk {
    topic: String,
    content: String,
}

#[derive(Deserialize, Debug)]
pub struct AugmentedFaq {
    id: usize,
    question: String,
}

#[derive(Deserialize, Debug)]
pub struct AugmentationResponse {
    augmented_faqs: Vec<AugmentedFaq>,
}

// --- Pipeline Orchestration ---

/// Orchestrates the full ingestion pipeline (Stages 1-3) for a given URL.
pub async fn run_ingestion_pipeline(
    db: &Database,
    ai_provider: &dyn AiProvider,
    url: &str,
) -> Result<usize, KnowledgeError> {
    let raw_content = match ingest_and_cache_url(db, url).await {
        Ok(content) => content,
        Err(KnowledgeError::ContentUnchanged(url)) => {
            info!("Content for {} is unchanged, pipeline finished.", url);
            return Ok(0);
        }
        Err(e) => return Err(e),
    };

    let faq_items = distill_and_augment(ai_provider, &raw_content).await?;
    store_structured_knowledge(db, &raw_content.url, &raw_content.content_hash, faq_items).await
}

// --- Stage 1: Ingestion & Caching ---

pub async fn create_kb_tables_if_not_exists(conn: &Connection) -> Result<(), turso::Error> {
    conn.execute(
        r#"CREATE TABLE IF NOT EXISTS raw_content (
            id INTEGER PRIMARY KEY AUTOINCREMENT, url TEXT UNIQUE NOT NULL, markdown_content TEXT NOT NULL,
            content_hash TEXT NOT NULL, last_fetched TEXT NOT NULL
        );"#,
        (),
    ).await?;
    conn.execute(
        r#"CREATE TABLE IF NOT EXISTS faq_kb (
            id INTEGER PRIMARY KEY AUTOINCREMENT, question TEXT NOT NULL, answer TEXT NOT NULL,
            source_url TEXT NOT NULL, is_explicit BOOLEAN NOT NULL, content_hash TEXT NOT NULL,
            last_modified TEXT NOT NULL, embedding BLOB
        );"#,
        (),
    )
    .await?;
    conn.execute(
        r#"CREATE TABLE IF NOT EXISTS refined_content (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_identifier TEXT NOT NULL,
            refined_markdown TEXT NOT NULL,
            raw_content_hash TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );"#,
        (),
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_refined_content_source_identifier ON refined_content(source_identifier);",
        (),
    ).await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_faq_kb_source_url ON faq_kb(source_url);",
        (),
    )
    .await?;
    Ok(())
}

pub async fn fetch_markdown_from_url(url: &str) -> Result<String, KnowledgeError> {
    let jina_url = format!("https://r.jina.ai/{url}");
    info!("Fetching clean markdown from: {jina_url}");
    let response = reqwest::get(&jina_url).await?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(KnowledgeError::JinaReaderFailed { status, body });
    }
    response.text().await.map_err(KnowledgeError::Fetch)
}

pub async fn ingest_and_cache_url(db: &Database, url: &str) -> Result<RawContent, KnowledgeError> {
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
        if let Ok(turso::Value::Text(existing_hash)) = row.get_value(0) {
            if existing_hash == content_hash {
                return Err(KnowledgeError::ContentUnchanged(url.to_string()));
            }
        }
    }

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

// --- Stage 2: Distillation & Augmentation (Batched) ---

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

        let batched_content = extracted_data
            .content_chunks
            .iter()
            .enumerate()
            .map(|(i, chunk)| {
                format!(
                    "---\nID: {}\nTOPIC: {}\nCONTENT:\n{}\n---\n",
                    i, chunk.topic, chunk.content
                )
            })
            .collect::<String>();

        let augmentation_user_prompt = format!("# Content Chunks to Analyze:\n{batched_content}");
        let llm_response = ai_provider
            .generate(AUGMENTATION_SYSTEM_PROMPT, &augmentation_user_prompt)
            .await?;

        match serde_json::from_str::<AugmentationResponse>(&llm_response) {
            Ok(parsed) => {
                let mut augmented_faqs = Vec::new();
                for aug_faq in parsed.augmented_faqs {
                    if let Some(original_chunk) = extracted_data.content_chunks.get(aug_faq.id) {
                        augmented_faqs.push(FaqItem {
                            question: aug_faq.question,
                            answer: original_chunk.content.clone(),
                            is_explicit: false,
                        });
                    }
                }
                info!(
                    "Pass 2 complete. Generated {} new FAQs from batch.",
                    augmented_faqs.len()
                );
                extracted_data.faqs.extend(augmented_faqs);
            }
            Err(e) => warn!(
                "Failed to parse batched augmentation response, skipping augmentation. Error: {}",
                e
            ),
        }
    }

    Ok(extracted_data.faqs)
}

// --- Stage 3: Structured Storage ---

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
    let mut stmt = conn.prepare(
        r#"INSERT INTO faq_kb (question, answer, source_url, is_explicit, content_hash, last_modified) VALUES (?, ?, ?, ?, ?, ?)"#,
    ).await?;

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
    info!(
        "Successfully stored {} new FAQs for URL: {url}",
        faq_items.len()
    );
    Ok(faq_items.len())
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

pub async fn export_for_finetuning(db: &Database) -> Result<String, KnowledgeError> {
    info!("Exporting knowledge base for fine-tuning.");
    let conn = db.connect()?;
    let mut stmt = conn.prepare("SELECT question, answer FROM faq_kb").await?;
    let mut rows = stmt.query(()).await?;
    let system_prompt = "You are a helpful assistant. Provide clear, accurate answers based on the retrieved context.";
    let mut jsonl_output = String::new();

    while let Some(row) = rows.next().await? {
        let question = if let Ok(turso::Value::Text(s)) = row.get_value(0) {
            s
        } else {
            continue;
        };
        let answer = if let Ok(turso::Value::Text(s)) = row.get_value(1) {
            s
        } else {
            continue;
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
