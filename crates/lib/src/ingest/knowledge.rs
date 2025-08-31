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
        KNOWLEDGE_EXTRACTION_USER_PROMPT, METADATA_EXTRACTION_SYSTEM_PROMPT,
    },
    providers::ai::AiProvider,
};
use md5;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};
use turso::{params, Database};
use uuid::Uuid;

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
    #[error("An internal error occurred: {0}")]
    Internal(#[from] anyhow::Error),
}

// --- Data Structures ---

/// Represents the essential data of a newly ingested or updated document.
#[derive(Debug, Clone)]
pub struct IngestedDocument {
    pub id: String,
    pub source_url: String,
    pub content: String,
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
    #[serde(default)]
    pub question: String,
    #[serde(default)]
    pub answer: String,
    pub is_explicit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ContentChunk {
    #[serde(default)]
    topic: String,
    #[serde(default)]
    content: String,
}

#[derive(Deserialize, Debug)]
pub struct AugmentedFaq {
    id: usize,
    #[serde(default)]
    question: String,
}

#[derive(Deserialize, Debug)]
pub struct AugmentationResponse {
    augmented_faqs: Vec<AugmentedFaq>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ContentMetadata {
    #[serde(rename = "type")]
    #[serde(default)]
    pub metadata_type: String,
    #[serde(default)]
    pub subtype: String,
    #[serde(default)]
    pub value: String,
}

// --- Pipeline Orchestration ---

/// Orchestrates the full ingestion pipeline (Stages 1-3) for a given URL.
pub async fn run_ingestion_pipeline(
    db: &Database,
    ai_provider: &dyn AiProvider,
    url: &str,
    owner_id: Option<&str>,
) -> Result<usize, KnowledgeError> {
    let (document_id, ingested_document) = match ingest_and_cache_url(db, url, owner_id).await {
        Ok(content) => content,
        Err(KnowledgeError::ContentUnchanged(url)) => {
            info!("Content for {} is unchanged, pipeline finished.", url);
            return Ok(0);
        }
        Err(e) => return Err(e),
    };

    // Run FAQ generation and metadata extraction concurrently.
    let (faq_result, metadata_result) = tokio::join!(
        distill_and_augment(ai_provider, &ingested_document),
        extract_and_store_metadata(
            db,
            ai_provider,
            &document_id,
            owner_id,
            &ingested_document.content
        )
    );

    // Handle results
    let faq_items = faq_result?;
    metadata_result?; // Propagate metadata errors

    store_structured_knowledge(db, &document_id, owner_id, faq_items).await
}

// --- Stage 1: Ingestion & Caching ---

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

pub async fn ingest_and_cache_url(
    db: &Database,
    url: &str,
    owner_id: Option<&str>,
) -> Result<(String, IngestedDocument), KnowledgeError> {
    let conn = db.connect()?;

    let markdown_content = fetch_markdown_from_url(url).await?;
    let new_content_hash = format!("{:x}", md5::compute(markdown_content.as_bytes()));

    // Check for existing document by source_url
    if let Some(row) = conn
        .query(
            "SELECT id, content FROM documents WHERE source_url = ?",
            params![url],
        )
        .await?
        .next()
        .await?
    {
        let doc_id: String = row.get(0)?;
        let existing_content: String = row.get(1)?;
        let existing_hash = format!("{:x}", md5::compute(existing_content.as_bytes()));

        if existing_hash == new_content_hash {
            return Err(KnowledgeError::ContentUnchanged(url.to_string()));
        }

        // Content has changed, so we update it.
        conn.execute(
            "UPDATE documents SET content = ? WHERE id = ?",
            params![markdown_content.clone(), doc_id.clone()],
        )
        .await?;

        info!("Successfully updated document for URL: {url}");
        let ingested_document = IngestedDocument {
            id: doc_id.clone(),
            source_url: url.to_string(),
            content: markdown_content,
            content_hash: new_content_hash,
        };
        return Ok((doc_id, ingested_document));
    }

    // No existing document, so create a new one.
    let document_id = Uuid::new_v4().to_string();
    let title: String = markdown_content.chars().take(80).collect();

    conn.execute(
        "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?, ?, ?, ?, ?)",
        params![
            document_id.clone(),
            owner_id,
            url,
            title,
            markdown_content.clone()
        ],
    )
    .await?;

    info!("Successfully stored new document for URL: {url}");
    let ingested_document = IngestedDocument {
        id: document_id.clone(),
        source_url: url.to_string(),
        content: markdown_content,
        content_hash: new_content_hash,
    };
    Ok((document_id, ingested_document))
}

// --- Stage 2: Distillation & Augmentation (Batched) ---

pub async fn distill_and_augment(
    ai_provider: &dyn AiProvider,
    ingested_doc: &IngestedDocument,
) -> Result<Vec<FaqItem>, KnowledgeError> {
    info!(
        "Starting Pass 1: Extraction for document ID: {}",
        ingested_doc.id
    );
    let user_prompt =
        KNOWLEDGE_EXTRACTION_USER_PROMPT.replace("{markdown_content}", &ingested_doc.content);
    let llm_response = ai_provider
        .generate(KNOWLEDGE_EXTRACTION_SYSTEM_PROMPT, &user_prompt)
        .await?;
    debug!("LLM extraction response: {}", llm_response);
    let cleaned_response = llm_response
        .trim()
        .strip_prefix("```json")
        .unwrap_or(&llm_response)
        .strip_suffix("```")
        .unwrap_or(&llm_response)
        .trim();
    let mut extracted_data: ExtractedKnowledge = serde_json::from_str(cleaned_response)?;
    let original_chunk_count = extracted_data.content_chunks.len();
    extracted_data
        .content_chunks
        .retain(|chunk| chunk.content.chars().any(|c| c.is_alphabetic()));
    let cleaned_chunk_count = extracted_data.content_chunks.len();

    if original_chunk_count > cleaned_chunk_count {
        info!(
            "Filtered out {} low-quality or separator-only chunks.",
            original_chunk_count - cleaned_chunk_count
        );
    }

    info!(
        "Pass 1 complete. Found {} explicit FAQs and {} valid content chunks.",
        extracted_data.faqs.len(),
        cleaned_chunk_count
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

        let cleaned_response = llm_response
            .trim()
            .strip_prefix("```json")
            .unwrap_or(&llm_response)
            .strip_suffix("```")
            .unwrap_or(&llm_response)
            .trim();
        match serde_json::from_str::<AugmentationResponse>(cleaned_response) {
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
    document_id: &str,
    owner_id: Option<&str>,
    faq_items: Vec<FaqItem>,
) -> Result<usize, KnowledgeError> {
    if faq_items.is_empty() {
        info!("No structured knowledge to store for document: {document_id}");
        return Ok(0);
    }
    let conn = db.connect()?;
    info!(
        "Storing {} FAQ items for document: {}",
        faq_items.len(),
        document_id
    );

    // We should delete old FAQs for this document before inserting new ones.
    conn.execute(
        "DELETE FROM faq_items WHERE document_id = ?",
        params![document_id],
    )
    .await?;
    conn.execute("BEGIN TRANSACTION", ()).await?;
    let mut stmt = conn
        .prepare(
            r#"INSERT INTO faq_items (document_id, owner_id, question, answer) VALUES (?, ?, ?, ?)"#,
        )
        .await?;

    for faq in &faq_items {
        stmt.execute(params![
            document_id,
            owner_id,
            faq.question.clone(),
            faq.answer.clone(),
        ])
        .await?;
    }

    conn.execute("COMMIT", ()).await?;
    info!(
        "Successfully stored {} new FAQs for document: {document_id}",
        faq_items.len()
    );
    Ok(faq_items.len())
}

// --- Stage 2.5: Hybrid Metadata Extraction ---

pub async fn extract_and_store_metadata(
    db: &Database,
    ai_provider: &dyn AiProvider,
    document_id: &str,
    owner_id: Option<&str>,
    content: &str,
) -> Result<(), KnowledgeError> {
    info!(
        "Starting metadata extraction for document ID: {}",
        document_id
    );

    let user_prompt = content;
    let llm_response = ai_provider
        .generate(METADATA_EXTRACTION_SYSTEM_PROMPT, user_prompt)
        .await?;

    debug!("LLM metadata response: {}", llm_response);
    let cleaned_response = llm_response
        .trim()
        .strip_prefix("```json")
        .unwrap_or(&llm_response)
        .strip_suffix("```")
        .unwrap_or(&llm_response)
        .trim();

    let metadata_items: Vec<ContentMetadata> = match serde_json::from_str(cleaned_response) {
        Ok(items) => items,
        Err(e) => {
            warn!(
                "Failed to parse metadata response, skipping metadata storage. Error: {}",
                e
            );
            return Ok(()); // Don't fail the whole pipeline if metadata fails
        }
    };

    if metadata_items.is_empty() {
        info!("No metadata extracted for document: {document_id}");
        return Ok(());
    }

    let conn = db.connect()?;
    info!(
        "Storing {} metadata items for document: {}",
        metadata_items.len(),
        document_id
    );

    // Clear old metadata for this document before inserting new items.
    conn.execute(
        "DELETE FROM content_metadata WHERE document_id = ?",
        params![document_id],
    )
    .await?;

    conn.execute("BEGIN TRANSACTION", ()).await?;
    let mut stmt = conn
        .prepare(
            r#"INSERT INTO content_metadata (document_id, owner_id, metadata_type, metadata_subtype, metadata_value)
               VALUES (?, ?, ?, ?, ?)"#,
        )
        .await?;

    for item in &metadata_items {
        stmt.execute(params![
            document_id,
            owner_id,
            item.metadata_type.to_uppercase(),
            item.subtype.clone(),
            item.value.clone(),
        ])
        .await?;
    }
    conn.execute("COMMIT", ()).await?;
    info!(
        "Successfully stored {} metadata items for document: {document_id}",
        metadata_items.len()
    );

    Ok(())
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
    let mut stmt = conn
        .prepare("SELECT question, answer FROM faq_items")
        .await?;
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
