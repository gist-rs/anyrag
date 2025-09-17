//! # `anyrag-text`: Raw Text Ingestion Plugin
//!
//! This crate provides the logic for ingesting raw text as a self-contained
//! plugin for the `anyrag` ecosystem. It implements the `Ingestor` trait from the
//! core `anyrag` library. It handles chunking the text and storing each chunk
//! as a separate document.

use anyhow::anyhow;
use anyrag::ingest::{IngestError as AnyragIngestError, IngestionResult, Ingestor};
use async_trait::async_trait;
use serde::Deserialize;
use thiserror::Error;
use tracing::warn;
use turso::{params, Connection, Database};
use uuid::Uuid;

/// The target maximum size for a single text chunk in characters.
const CHUNK_SIZE_LIMIT: usize = 4096;
/// The character overlap to include between consecutive chunks.
const CHUNK_OVERLAP: usize = 200;

/// Custom error types for the text ingestion process.
#[derive(Error, Debug)]
pub enum TextIngestError {
    #[error("Text content is empty or only whitespace")]
    EmptyContent,
    #[error("Database error: {0}")]
    Database(#[from] turso::Error),
    #[error("Source deserialization failed: {0}")]
    SourceDeserialization(#[from] serde_json::Error),
}

/// A helper to convert the specific `TextIngestError` into the generic `anyrag::ingest::IngestError`.
impl From<TextIngestError> for AnyragIngestError {
    fn from(err: TextIngestError) -> Self {
        match err {
            TextIngestError::Database(e) => AnyragIngestError::Database(e),
            TextIngestError::EmptyContent => {
                AnyragIngestError::Parse("Text content is empty or only whitespace".to_string())
            }
            TextIngestError::SourceDeserialization(e) => {
                AnyragIngestError::Internal(anyhow!("Failed to deserialize source JSON: {}", e))
            }
        }
    }
}

/// Defines the structure of the JSON string passed to the `ingest` method.
#[derive(Deserialize)]
struct TextSource {
    text: String,
    source: String,
}

/// The `Ingestor` implementation for raw text.
pub struct TextIngestor<'a> {
    db: &'a Database,
}

impl<'a> TextIngestor<'a> {
    /// Creates a new `TextIngestor`.
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl Ingestor for TextIngestor<'_> {
    /// Ingests a block of raw text.
    ///
    /// The `source` argument is expected to be a JSON string with `text` and `source`
    /// keys, for example:
    /// `{"text": "This is the content.", "source": "manual_input"}`.
    async fn ingest(
        &self,
        source: &str,
        owner_id: Option<&str>,
    ) -> Result<IngestionResult, AnyragIngestError> {
        let text_source: TextSource =
            serde_json::from_str(source).map_err(TextIngestError::from)?;
        let chunks = chunk_text(&text_source.text)?;
        let mut conn = self.db.connect().map_err(TextIngestError::from)?;
        let document_ids =
            ingest_chunks_as_documents(&mut conn, chunks, &text_source.source, owner_id).await?;

        Ok(IngestionResult {
            documents_added: document_ids.len(),
            source: text_source.source,
            document_ids,
        })
    }
}

/// Chunks a given text into smaller pieces based on paragraphs and size limits.
pub fn chunk_text(text: &str) -> Result<Vec<String>, TextIngestError> {
    let trimmed_text = text.trim();
    if trimmed_text.is_empty() {
        return Err(TextIngestError::EmptyContent);
    }

    let mut chunks = Vec::new();
    let paragraphs = trimmed_text.split("\n\n");

    for paragraph in paragraphs {
        let p_trimmed = paragraph.trim();
        if p_trimmed.is_empty() {
            continue;
        }

        if p_trimmed.chars().count() <= CHUNK_SIZE_LIMIT {
            chunks.push(p_trimmed.to_string());
        } else {
            warn!(
                "Paragraph exceeds chunk size limit ({} > {}). Splitting by character.",
                p_trimmed.chars().count(),
                CHUNK_SIZE_LIMIT
            );
            let mut sub_chunks = split_long_text(p_trimmed);
            chunks.append(&mut sub_chunks);
        }
    }

    Ok(chunks)
}

/// Takes a vector of text chunks and ingests them into the `documents` table.
pub async fn ingest_chunks_as_documents(
    conn: &mut Connection,
    chunks: Vec<String>,
    source_identifier: &str,
    owner_id: Option<&str>,
) -> Result<Vec<String>, TextIngestError> {
    if chunks.is_empty() {
        return Ok(Vec::new());
    }

    let tx = conn.transaction().await?;
    let mut new_document_ids = Vec::new();

    for (i, chunk) in chunks.iter().enumerate() {
        let document_id = Uuid::new_v4().to_string();
        // Create a unique source URL for each chunk to avoid collisions.
        let source_url = format!("{source_identifier}#chunk_{i}");
        let title: String = chunk.chars().take(80).collect();

        tx.execute(
            "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?, ?, ?, ?, ?)",
            params![
                document_id.clone(),
                owner_id,
                source_url,
                title,
                chunk.clone()
            ],
        ).await?;
        new_document_ids.push(document_id);
    }

    tx.commit().await?;

    Ok(new_document_ids)
}

/// Splits a long string into chunks that are at most `CHUNK_SIZE_LIMIT` characters long.
fn split_long_text(text: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut start = 0;

    while start < chars.len() {
        let end = std::cmp::min(start + CHUNK_SIZE_LIMIT, chars.len());
        let chunk: String = chars[start..end].iter().collect();
        chunks.push(chunk);

        // Move the start for the next chunk, considering the overlap.
        let next_start = start + CHUNK_SIZE_LIMIT - CHUNK_OVERLAP;
        if next_start >= chars.len() || next_start <= start {
            break;
        }
        start = next_start;
    }

    chunks
}
