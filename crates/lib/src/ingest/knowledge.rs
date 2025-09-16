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

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};
use turso::{params, Connection, Database};
use uuid::Uuid;

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

/// The public-facing, validated struct for a Q&A pair.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FaqItem {
    pub question: String,
    pub answer: String,
    pub is_explicit: bool,
}

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
pub struct MetadataResponse {
    #[serde(default)]
    metadata: Vec<ContentMetadata>,
}

/// A struct to hold the prompts for the various LLM calls in the ingestion pipeline.
pub struct IngestionPrompts<'a> {
    pub extraction_system_prompt: &'a str,
    pub extraction_user_prompt_template: &'a str,
    pub augmentation_system_prompt: &'a str,
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
    let (document_id, ingested_document) =
        match ingest_and_cache_url(db, url, owner_id, web_ingest_strategy).await {
            Ok(content) => content,
            Err(KnowledgeError::ContentUnchanged(url)) => {
                info!("Content for {} is unchanged, pipeline finished.", url);
                return Ok(0);
            }
            Err(e) => return Err(e),
        };

    let conn = db.connect()?;

    // --- Sequential Execution for better metadata ---

    // 1. First, distill the raw content into structured FAQs.
    let faq_items = distill_and_augment(
        ai_provider,
        &ingested_document,
        prompts.extraction_system_prompt,
        prompts.extraction_user_prompt_template,
        prompts.augmentation_system_prompt,
    )
    .await?;

    // 2. Then, extract metadata from the *clean* FAQ data, not the noisy source.
    let content_for_metadata = if faq_items.is_empty() {
        // If no FAQs, fall back to the original content for metadata extraction.
        ingested_document.content.clone()
    } else {
        faq_items
            .iter()
            .map(|faq| format!("Q: {}\nA: {}", faq.question, faq.answer))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")
    };

    let metadata_items = extract_and_store_metadata(
        &conn,
        ai_provider,
        &document_id,
        owner_id,
        &content_for_metadata,
        prompts.metadata_extraction_system_prompt,
    )
    .await?;

    let count = store_faqs_as_documents(
        db,
        &ingested_document.source_url,
        owner_id,
        &faq_items,
        &metadata_items,
    )
    .await?;

    // Set the original document's content to an empty string to exclude it from search results,
    // while preserving its row for caching purposes.
    if count > 0 {
        conn.execute(
            "UPDATE documents SET content = ? WHERE id = ?",
            params!["", document_id.clone()],
        )
        .await?;
        info!(
            "Cleared content of source document '{}' after extracting FAQs.",
            document_id
        );
    }

    Ok(count)
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

// --- Stage 2: Distillation & Augmentation (Batched) ---

pub async fn distill_and_augment(
    ai_provider: &dyn AiProvider,
    ingested_doc: &IngestedDocument,
    extraction_system_prompt: &str,
    extraction_user_prompt_template: &str,
    augmentation_system_prompt: &str,
) -> Result<Vec<FaqItem>, KnowledgeError> {
    info!(
        "Starting Pass 1: Extraction for document ID: {}",
        ingested_doc.id
    );

    // Define temporary structs that can handle nullable fields from the LLM.
    // This makes the initial parsing robust against `null` values.
    #[derive(Deserialize)]
    struct RawFaqItem {
        #[serde(default)]
        question: Option<String>,
        #[serde(default)]
        answer: Option<String>,
        is_explicit: bool,
    }

    #[derive(Deserialize)]
    struct RawExtractedKnowledge {
        #[serde(default)]
        faqs: Vec<RawFaqItem>,
        #[serde(default)]
        content_chunks: Vec<ContentChunk>,
    }

    let user_prompt =
        extraction_user_prompt_template.replace("{markdown_content}", &ingested_doc.content);
    let llm_response = ai_provider
        .generate(extraction_system_prompt, &user_prompt)
        .await?;
    debug!("LLM extraction response: {}", llm_response);
    let cleaned_response = clean_llm_response(&llm_response);
    let mut extracted_data: RawExtractedKnowledge = match serde_json::from_str(&cleaned_response) {
        Ok(data) => data,
        Err(e) => {
            warn!(
                "Failed to parse extraction response JSON. Error: {}. Raw response: '{}'",
                e, &cleaned_response
            );
            return Err(e.into());
        }
    };

    // Filter and map the raw, potentially noisy data into the clean, validated FaqItem struct.
    // This removes any items where the question or answer was null, missing, or just an empty string.
    let mut clean_faqs: Vec<FaqItem> = extracted_data
        .faqs
        .into_iter()
        .filter_map(|raw_faq| {
            match (raw_faq.question, raw_faq.answer) {
                (Some(q), Some(a)) if !q.trim().is_empty() && !a.trim().is_empty() => {
                    Some(FaqItem {
                        question: q,
                        answer: a,
                        is_explicit: raw_faq.is_explicit,
                    })
                }
                _ => None, // Discard if question or answer is None or empty
            }
        })
        .collect();

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
        clean_faqs.len(),
        cleaned_chunk_count
    );

    if extracted_data.content_chunks.is_empty() {
        info!("No content chunks found, skipping augmentation step.");
    } else {
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
            .generate(augmentation_system_prompt, &augmentation_user_prompt)
            .await?;

        let cleaned_response = clean_llm_response(&llm_response);
        match serde_json::from_str::<AugmentationResponse>(&cleaned_response) {
            Ok(parsed) => {
                let mut augmented_faqs = Vec::new();
                for aug_faq in parsed.augmented_faqs {
                    if let Some(original_chunk) = extracted_data.content_chunks.get(aug_faq.id) {
                        if !aug_faq.question.trim().is_empty() {
                            augmented_faqs.push(FaqItem {
                                question: aug_faq.question,
                                answer: original_chunk.content.clone(),
                                is_explicit: false,
                            });
                        }
                    }
                }
                info!(
                    "Pass 2 complete. Generated {} new FAQs from batch.",
                    augmented_faqs.len()
                );
                clean_faqs.extend(augmented_faqs);
            }
            Err(e) => warn!(
                "Failed to parse batched augmentation response, skipping augmentation. Error: {}",
                e
            ),
        }
    }

    Ok(clean_faqs)
}

// --- Stage 3: Structured Storage ---

pub async fn store_faqs_as_documents(
    db: &Database,
    parent_source_url: &str,
    owner_id: Option<&str>,
    faq_items: &[FaqItem],
    metadata: &[ContentMetadata],
) -> Result<usize, KnowledgeError> {
    if faq_items.is_empty() {
        info!("No FAQs to store as documents for source: {parent_source_url}");
        return Ok(0);
    }
    let conn = db.connect()?;
    info!(
        "Storing {} FAQs as individual documents for source: {}",
        faq_items.len(),
        parent_source_url
    );

    // First, remove old FAQ documents derived from this parent source URL to ensure idempotency.
    let url_pattern = format!("{parent_source_url}#faq_%");
    conn.execute(
        "DELETE FROM documents WHERE source_url LIKE ?",
        params![url_pattern],
    )
    .await?;

    conn.execute("BEGIN TRANSACTION", ()).await?;
    let mut stmt = conn
        .prepare(
            r#"INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?, ?, ?, ?, ?)"#,
        )
        .await?;

    let mut new_document_ids = Vec::new();
    for (i, faq) in faq_items.iter().enumerate() {
        // Create a stable, unique ID and a unique source URL for each FAQ document.
        let faq_source_url = format!("{parent_source_url}#faq_{i}");
        let document_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, faq_source_url.as_bytes()).to_string();
        new_document_ids.push(document_id.clone());

        stmt.execute(params![
            document_id,
            owner_id,
            faq_source_url,
            faq.question.clone(), // The question becomes the title
            faq.answer.clone(),   // The answer becomes the content
        ])
        .await?;
    }

    // --- Propagate Metadata ---
    // If metadata was extracted from the parent, associate it with all the new FAQ documents.
    if !metadata.is_empty() && !new_document_ids.is_empty() {
        info!(
            "Propagating {} metadata items to {} new FAQ documents.",
            metadata.len(),
            new_document_ids.len()
        );
        let mut meta_stmt = conn
            .prepare(
                "INSERT INTO content_metadata (document_id, owner_id, metadata_type, metadata_subtype, metadata_value) VALUES (?, ?, ?, ?, ?)",
            )
            .await?;

        for doc_id in &new_document_ids {
            for item in metadata {
                meta_stmt
                    .execute(params![
                        doc_id.to_string(),
                        owner_id.map(|s| s.to_string()),
                        item.metadata_type.to_uppercase(),
                        item.subtype.clone(),
                        item.value.clone(),
                    ])
                    .await?;
            }
        }
    }

    conn.execute("COMMIT", ()).await?;
    info!(
        "Successfully stored {} new FAQ documents for source: {parent_source_url}",
        faq_items.len()
    );
    Ok(faq_items.len())
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

    let metadata_json = serde_json::to_string_pretty(&metadata_items)
        .unwrap_or_else(|_| "Failed to serialize metadata".to_string());
    info!("Extracted metadata for document {document_id}: {metadata_json}");

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
    info!("Exporting knowledge base for fine-tuning.");
    let conn = db.connect()?;
    let mut stmt = conn
        .prepare("SELECT title, content FROM documents WHERE source_url LIKE '%#faq_%'")
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
