//! # Knowledge Base Utilities
//!
//! This module provides shared utilities for the knowledge base, such as helper
//! functions for cleaning LLM responses and logic for exporting data for fine-tuning.
//! The core ingestion pipelines are now located in their respective plugin crates
//! (e.g., `anyrag-web`, `anyrag-pdf`).

use crate::ingest::types::{ContentMetadata, MetadataResponse};
use crate::providers::ai::AiProvider;
use crate::PromptError;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};
use turso::{params, Connection, Database};

// --- Data Structures for YAML Parsing ---
// These structs are used for parsing the structured YAML content stored in the documents table,
// particularly for the fine-tuning export functionality.

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Faq {
    pub question: String,
    pub answer: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Section {
    pub title: String,
    pub faqs: Vec<Faq>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct YamlContent {
    pub sections: Vec<Section>,
}

// --- Error Definition ---

#[derive(Error, Debug)]
pub enum KnowledgeError {
    #[error("Database error: {0}")]
    Database(#[from] turso::Error),
    #[error("Failed to parse or serialize data: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("LLM processing failed: {0}")]
    Llm(#[from] PromptError),
}

// --- Helper Functions ---

/// Cleans the raw JSON response from an LLM, removing markdown code fences.
pub fn clean_llm_response(response: &str) -> String {
    response
        .trim()
        .strip_prefix("```json")
        .unwrap_or(response)
        .strip_suffix("```")
        .unwrap_or(response)
        .trim()
        .replace("\\*", "*")
}

// --- Fine-Tuning Export ---

#[derive(Serialize, Debug)]
struct FinetuningEntry<'a> {
    messages: Vec<FinetuningMessage<'a>>,
}

#[derive(Serialize, Debug)]
struct FinetuningMessage<'a> {
    role: &'a str,
    content: &'a str,
}

/// Exports the structured knowledge base into a JSONL file suitable for fine-tuning models.
pub async fn export_for_finetuning(db: &Database) -> Result<String, KnowledgeError> {
    info!("Exporting knowledge base for fine-tuning from structured YAML.");
    let conn = db.connect()?;
    let mut stmt = conn
        .prepare("SELECT content FROM documents WHERE content IS NOT NULL AND content != ''")
        .await?;
    let mut rows = stmt.query(()).await?;
    let system_prompt =
        "You are a helpful assistant. Provide clear, accurate answers based on the retrieved context.";
    let mut jsonl_output = String::new();

    while let Some(row) = rows.next().await? {
        let yaml_content = if let Ok(turso::Value::Text(s)) = row.get_value(0) {
            s
        } else {
            continue;
        };

        // Only process documents that appear to be our structured YAML format.
        if !yaml_content.trim().starts_with("sections:") {
            continue;
        }

        let parsed_yaml: YamlContent = match serde_yaml::from_str(&yaml_content) {
            Ok(data) => data,
            Err(e) => {
                warn!("Failed to parse YAML content for fine-tuning export, skipping document. Error: {}", e);
                continue;
            }
        };

        for section in parsed_yaml.sections {
            for faq in section.faqs {
                let entry = FinetuningEntry {
                    messages: vec![
                        FinetuningMessage {
                            role: "system",
                            content: system_prompt,
                        },
                        FinetuningMessage {
                            role: "user",
                            content: &faq.question,
                        },
                        FinetuningMessage {
                            role: "assistant",
                            content: &faq.answer,
                        },
                    ],
                };
                let line = serde_json::to_string(&entry)?;
                jsonl_output.push_str(&line);
                jsonl_output.push('\n');
            }
        }
    }

    info!(
        "Generated fine-tuning data with {} entries.",
        jsonl_output.lines().count()
    );
    Ok(jsonl_output)
}

// --- Core Ingestion Pipeline Functions ---

pub async fn restructure_with_llm(
    ai_provider: &dyn AiProvider,
    markdown_content: &str,
    system_prompt: &str,
) -> Result<String, KnowledgeError> {
    let user_prompt = format!("# Markdown Content to Process:\n{markdown_content}");
    let llm_response = ai_provider.generate(system_prompt, &user_prompt).await?;
    let cleaned_yaml = llm_response
        .trim()
        .strip_prefix("```yaml")
        .unwrap_or(&llm_response)
        .strip_suffix("```")
        .unwrap_or(&llm_response)
        .trim();
    Ok(cleaned_yaml.to_string())
}

pub async fn extract_and_store_metadata(
    conn: &Connection,
    ai_provider: &dyn AiProvider,
    document_id: &str,
    owner_id: Option<&str>,
    content: &str,
    system_prompt: &str,
) -> Result<(), KnowledgeError> {
    let user_prompt = content;
    let llm_response = ai_provider.generate(system_prompt, user_prompt).await?;
    debug!("LLM metadata response: {}", llm_response);
    let cleaned_response = clean_llm_response(&llm_response);

    let metadata_items: Vec<ContentMetadata> =
        if let Ok(items) = serde_json::from_str(&cleaned_response) {
            items
        } else if let Ok(response) = serde_json::from_str::<MetadataResponse>(&cleaned_response) {
            response.metadata
        } else {
            warn!(
                "Failed to parse metadata response, skipping. Raw response: '{}'",
                &cleaned_response
            );
            return Ok(());
        };

    conn.execute(
        "DELETE FROM content_metadata WHERE document_id = ?",
        params![document_id],
    )
    .await?;

    if metadata_items.is_empty() {
        return Ok(());
    }

    conn.execute("BEGIN TRANSACTION", ()).await?;
    let mut stmt = conn.prepare("INSERT INTO content_metadata (document_id, owner_id, metadata_type, metadata_subtype, metadata_value) VALUES (?, ?, ?, ?, ?)")
        .await?;
    for item in &metadata_items {
        stmt.execute(params![
            document_id.to_string(),
            owner_id.map(|s| s.to_string()),
            item.metadata_type.to_uppercase(),
            item.subtype.clone(),
            item.value.clone()
        ])
        .await?;
    }
    conn.execute("COMMIT", ()).await?;
    Ok(())
}
