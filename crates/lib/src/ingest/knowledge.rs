//! # Knowledge Base Ingestion Pipeline
//!
//! This module implements the 5-stage virtuous cycle for building a knowledge base:
//! 1.  **Ingestion & Caching**: Fetches raw web content and stores it, avoiding reprocessing.
//! 2.  **Distillation & Augmentation**: Uses an LLM to extract explicit FAQs and generate new ones.
//! 3.  **Structured Storage**: Stores the structured data in SQLite for retrieval.
//! 4.  **Hybrid Retrieval**: The RAG query process (implemented elsewhere).
//! 5.  **Fine-Tuning Export**: Exports the knowledge base into a format for model fine-tuning.

use crate::{errors::PromptError, providers::ai::AiProvider};
use html;

use md5;
use serde_yaml;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};
use turso::{params, Connection, Database};
use uuid::Uuid;

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

/// Defines the strategy for fetching web content.
#[derive(Debug, Clone, Copy, Default)]
pub enum WebIngestStrategy<'a> {
    /// Fetch raw HTML and convert it to Markdown. This is the default.
    #[default]
    RawHtml,
    /// Use the Jina Reader API to get clean Markdown.
    Jina { api_key: Option<&'a str> },
}

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
    #[error("HTML processing error: {0}")]
    Html(String),
    #[error("Content appears to be contaminated with forbidden HTML tags after cleaning: {0}")]
    ContaminatedContent(String),
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
pub struct ContentMetadata {
    #[serde(rename = "type")]
    #[serde(default)]
    pub metadata_type: String,
    #[serde(default)]
    pub subtype: String,
    #[serde(default)]
    pub value: String,
}

#[derive(Deserialize, Debug)]
pub struct MetadataResponse {
    #[serde(default)]
    metadata: Vec<ContentMetadata>,
}

/// A struct to hold the prompts for the various LLM calls in the ingestion pipeline.
pub struct IngestionPrompts<'a> {
    pub restructuring_system_prompt: &'a str,
    pub metadata_extraction_system_prompt: &'a str,
}

// --- Pipeline Orchestration ---

/// Orchestrates the full ingestion pipeline (Stages 1-3) for a given URL.
pub async fn run_ingestion_pipeline(
    db: &Database,
    ai_provider: &dyn AiProvider,
    url: &str,
    owner_id: Option<&str>,
    prompts: IngestionPrompts<'_>,
    web_ingest_strategy: WebIngestStrategy<'_>,
) -> Result<usize, KnowledgeError> {
    // Stage 1: Ingest Raw Content
    let (temp_doc_id, ingested_document) =
        match ingest_and_cache_url(db, url, owner_id, web_ingest_strategy).await {
            Ok(content) => content,
            Err(KnowledgeError::ContentUnchanged(url)) => {
                info!("Content for {} is unchanged, pipeline finished.", url);
                return Ok(0);
            }
            Err(e) => return Err(e),
        };

    // Stage 2: Restructure to YAML
    let structured_yaml = restructure_with_llm(
        ai_provider,
        &ingested_document.content,
        prompts.restructuring_system_prompt,
    )
    .await?;

    if structured_yaml.trim().is_empty() || structured_yaml.trim() == "[]" {
        warn!(
            "LLM restructuring resulted in empty content for source: {}",
            url
        );
        // Clean up the temporary document
        let conn = db.connect()?;
        conn.execute("DELETE FROM documents WHERE id = ?", params![temp_doc_id])
            .await?;
        return Ok(0);
    }

    let conn = db.connect()?;

    // Stage 3: Chunk from YAML and Store
    // Delete the original raw document, it's no longer needed.
    conn.execute(
        "DELETE FROM documents WHERE id = ?",
        params![temp_doc_id.clone()],
    )
    .await?;
    info!("Deleted temporary raw document with id: {}", temp_doc_id);

    let yaml_content: YamlContent = match serde_yaml::from_str(&structured_yaml) {
        Ok(content) => content,
        Err(e) => {
            warn!(
                "Failed to parse structured YAML for source: {}. Error: {}",
                url, e
            );
            return Ok(0); // Or return an error
        }
    };

    let mut chunks_created = 0;
    for (i, section) in yaml_content.sections.into_iter().enumerate() {
        // 1. Create a new YamlContent object containing only the current section.
        let chunk_content = YamlContent {
            sections: vec![section.clone()],
        };

        // 2. Serialize this new object back into a small, self-contained yaml_chunk string.
        let yaml_chunk = match serde_yaml::to_string(&chunk_content) {
            Ok(s) => s,
            Err(_) => continue, // Skip if serialization fails
        };

        // 3. Generate a unique chunk_id for the new document.
        let chunk_id = Uuid::new_v4().to_string();
        let source_url_with_chunk = format!("{url}#section_{i}");

        // 4. INSERT the yaml_chunk into the documents table.
        conn.execute(
            "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?, ?, ?, ?, ?)",
            params![
                chunk_id.clone(),
                owner_id,
                source_url_with_chunk, // The original source URL
                section.title.clone(),
                yaml_chunk.clone()
            ],
        )
        .await?;
        info!(
            "Stored YAML chunk '{}' for document id: {}",
            section.title, chunk_id
        );
        chunks_created += 1;

        // Stage 4: Extract Metadata for the new chunk
        extract_and_store_metadata(
            &conn,
            ai_provider,
            &chunk_id,
            owner_id,
            &yaml_chunk,
            prompts.metadata_extraction_system_prompt,
        )
        .await?;
    }

    Ok(chunks_created)
}

// --- Stage 1: Ingestion & Caching ---

pub async fn fetch_web_content(
    url: &str,
    strategy: WebIngestStrategy<'_>,
) -> Result<String, KnowledgeError> {
    match strategy {
        WebIngestStrategy::RawHtml => {
            info!("Fetching and cleaning HTML from: {url}");
            html::url_to_clean_markdown(url, None)
                .await
                .map_err(|e| KnowledgeError::Html(e.to_string()))
        }
        WebIngestStrategy::Jina { api_key } => {
            let fetch_url = format!("https://r.jina.ai/{url}");
            info!("Fetching clean markdown from: {fetch_url}");

            let client = reqwest::Client::new();
            let mut request_builder = client.get(&fetch_url);

            if let Some(key) = api_key {
                if !key.is_empty() {
                    info!("Using Jina API key for request.");
                    request_builder =
                        request_builder.header("Authorization", format!("Bearer {key}"));
                }
            } else {
                warn!("No Jina API key provided. You may be subject to a 20 RPM rate limit.");
            }

            let response = request_builder.send().await?;
            if !response.status().is_success() {
                let status = response.status().as_u16();
                let body = response.text().await.unwrap_or_default();
                return Err(KnowledgeError::JinaReaderFailed { status, body });
            }
            let markdown = response.text().await.map_err(KnowledgeError::Fetch)?;
            Ok(html::clean_markdown_content(&markdown))
        }
    }
}

pub async fn ingest_and_cache_url(
    db: &Database,
    url: &str,
    owner_id: Option<&str>,
    strategy: WebIngestStrategy<'_>,
) -> Result<(String, IngestedDocument), KnowledgeError> {
    let conn = db.connect()?;

    let markdown_content = fetch_web_content(url, strategy).await?;
    let new_content_hash = format!("{:x}", md5::compute(markdown_content.as_bytes()));

    // Extract a cleaner title from the markdown content.
    let title = markdown_content
        .lines()
        .find(|line| !line.trim().is_empty()) // Find the first non-empty line
        .map(|line| {
            line.trim_start_matches(|c: char| c == '#' || c.is_whitespace()) // Remove markdown headings and leading spaces
                .trim_start_matches("Title:") // Remove "Title:" prefix
                .trim() // Trim whitespace
                .chars()
                .take(150) // Take up to 150 characters for the title
                .collect::<String>()
        })
        .unwrap_or_else(|| url.to_string()); // Fallback to URL

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

        // Content has changed, so we update it, along with the title.
        conn.execute(
            "UPDATE documents SET content = ?, title = ? WHERE id = ?",
            params![markdown_content.clone(), title, doc_id.clone()],
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
    let document_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, url.as_bytes()).to_string();

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

// --- Stage 2: LLM-Powered Restructuring ---

/// Uses an LLM to restructure messy Markdown into a clean, structured YAML format.
pub async fn restructure_with_llm(
    ai_provider: &dyn AiProvider,
    markdown_content: &str,
    system_prompt: &str,
) -> Result<String, KnowledgeError> {
    info!("Starting LLM-powered restructuring of Markdown content.");

    // The user prompt is simply the markdown content, as the system prompt contains all instructions.
    let user_prompt = format!("# Markdown Content to Process:\n{markdown_content}");

    let llm_response = ai_provider.generate(system_prompt, &user_prompt).await?;

    // The response should be the raw YAML, so we just clean the code fences if they exist.
    let cleaned_yaml = llm_response
        .trim()
        .strip_prefix("```yaml")
        .unwrap_or(&llm_response)
        .strip_suffix("```")
        .unwrap_or(&llm_response)
        .trim();

    info!("Successfully restructured content into YAML format.");
    Ok(cleaned_yaml.to_string())
}

// --- Stage 2.5: Hybrid Metadata Extraction ---

pub async fn extract_and_store_metadata(
    conn: &Connection,
    ai_provider: &dyn AiProvider,
    document_id: &str,
    owner_id: Option<&str>,
    content: &str,
    system_prompt: &str,
) -> Result<Vec<ContentMetadata>, KnowledgeError> {
    info!(
        "Starting metadata extraction for document ID: {}",
        document_id
    );

    let user_prompt = content;
    let llm_response = ai_provider.generate(system_prompt, user_prompt).await?;

    debug!("LLM metadata response: {}", llm_response);
    let cleaned_response = clean_llm_response(&llm_response);

    let parsed_metadata: Vec<ContentMetadata> =
        if let Ok(items) = serde_json::from_str(&cleaned_response) {
            items
        } else if let Ok(response) = serde_json::from_str::<MetadataResponse>(&cleaned_response) {
            response.metadata
        } else {
            warn!(
            "Failed to parse metadata response as array or object, skipping. Raw response: '{}'",
            &cleaned_response
        );
            return Ok(Vec::new());
        };

    // --- Programmatic Filtering and Limiting (Safety Net) ---
    let mut categories = Vec::new();
    let mut keyphrases = Vec::new();
    let mut entities = Vec::new();

    for item in parsed_metadata {
        // Rule 1: Filter out forbidden generic user identifiers.
        if item.value.starts_with("สมาชิกหมายเลข") {
            continue;
        }

        // Rule 2: Group by type.
        match item.metadata_type.to_uppercase().as_str() {
            "CATEGORY" => categories.push(item),
            "KEYPHRASE" => keyphrases.push(item),
            "ENTITY" => entities.push(item),
            _ => (), // Ignore unknown types
        }
    }

    // Rule 3: Truncate each list to the desired limit.
    categories.truncate(1);
    keyphrases.truncate(10);
    entities.truncate(10);

    // Combine the capped lists back into one.
    let mut metadata_items = Vec::new();
    metadata_items.append(&mut categories);
    metadata_items.append(&mut keyphrases);
    metadata_items.append(&mut entities);

    let _metadata_json = serde_json::to_string_pretty(&metadata_items)
        .unwrap_or_else(|_| "Failed to serialize metadata".to_string());
    // info!("Extracted metadata for document {document_id}: {metadata_json}");

    if metadata_items.is_empty() {
        info!("No metadata extracted for document: {document_id}");
        return Ok(metadata_items);
    }

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

    // Use a transaction with individual inserts for better stability.
    conn.execute("BEGIN TRANSACTION", ()).await?;
    let mut stmt = conn
        .prepare(
            "INSERT INTO content_metadata (document_id, owner_id, metadata_type, metadata_subtype, metadata_value) VALUES (?, ?, ?, ?, ?)",
        )
        .await?;

    for item in &metadata_items {
        stmt.execute(params![
            document_id.to_string(),
            owner_id.map(|s| s.to_string()),
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

    Ok(metadata_items)
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
    info!("Exporting knowledge base for fine-tuning from structured YAML.");
    let conn = db.connect()?;
    let mut stmt = conn
        .prepare(
            "SELECT content FROM documents WHERE source_url NOT LIKE '%#faq_%' AND content IS NOT NULL AND content != ''",
        )
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
