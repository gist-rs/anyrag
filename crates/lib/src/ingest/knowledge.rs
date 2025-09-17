//! # Knowledge Base Utilities
//!
//! This module provides shared utilities for the knowledge base, such as helper
//! functions for cleaning LLM responses and logic for exporting data for fine-tuning.
//! The core ingestion pipelines are now located in their respective plugin crates
//! (e.g., `anyrag-web`, `anyrag-pdf`).

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn};
use turso::Database;

// --- Data Structures for YAML Parsing ---
// These structs are used for parsing the structured YAML content stored in the documents table,
// particularly for the fine-tuning export functionality.

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Faq {
    question: String,
    answer: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Section {
    title: String,
    faqs: Vec<Faq>,
}

#[derive(Debug, Deserialize, Serialize)]
struct YamlContent {
    sections: Vec<Section>,
}

// --- Error Definition ---

#[derive(Error, Debug)]
pub enum KnowledgeError {
    #[error("Database error: {0}")]
    Database(#[from] turso::Error),
    #[error("Failed to parse or serialize data: {0}")]
    Parse(#[from] serde_json::Error),
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
