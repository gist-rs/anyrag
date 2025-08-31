//! # Text Ingestion and Chunking
//!
//! This module provides the core logic for ingesting raw text data,
//! splitting it into manageable chunks, and preparing it for embedding
//! and storage.

use thiserror::Error;
use tracing::warn;
use turso::{params, Connection};
use uuid::Uuid;

/// The target maximum size for a single text chunk in characters.
/// This is set conservatively to leave room for prompt formatting and to
/// work well with a wide range of embedding models.
const CHUNK_SIZE_LIMIT: usize = 4096;

/// The character overlap to include between consecutive chunks.
/// This helps maintain semantic context across chunk boundaries.
const CHUNK_OVERLAP: usize = 200;

#[derive(Error, Debug)]
pub enum IngestError {
    #[error("Text content is empty or only whitespace")]
    EmptyContent,
    #[error("Database error: {0}")]
    Database(#[from] turso::Error),
}

/// Chunks a given text into smaller pieces based on paragraphs and size limits.
///
/// The chunking strategy is as follows:
/// 1. Split the text into paragraphs using `\n\n`.
/// 2. For each paragraph:
///    a. If it's smaller than `CHUNK_SIZE_LIMIT`, it's considered a chunk.
///    b. If it's larger, it's recursively split by character length, ensuring
///    that no chunk exceeds the limit. An overlap is maintained between
///    these splits to preserve context.
///
/// This approach prioritizes semantic boundaries (paragraphs) while still
/// handling very long paragraphs gracefully.
///
/// # Arguments
///
/// * `text` - The raw text content to be chunked.
///
/// # Returns
///
/// A `Result` containing a `Vec<String>` of text chunks, or an `IngestError`
/// if the input text is empty or only whitespace.
pub fn chunk_text(text: &str) -> Result<Vec<String>, IngestError> {
    let trimmed_text = text.trim();
    if trimmed_text.is_empty() {
        return Err(IngestError::EmptyContent);
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
            // If a paragraph is too long, split it by character length with overlap.
            let mut sub_chunks = split_long_text(p_trimmed);
            chunks.append(&mut sub_chunks);
        }
    }

    Ok(chunks)
}

/// Takes a vector of text chunks and ingests them into the `documents` table.
/// Each chunk becomes a separate document.
///
/// # Returns
///
/// A `Result` containing a vector of the UUIDs of the newly created documents.
pub async fn ingest_chunks_as_documents(
    conn: &mut Connection,
    chunks: Vec<String>,
    source_identifier: &str,
    owner_id: Option<&str>,
) -> Result<Vec<String>, IngestError> {
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

/// Splits a long string into chunks that are at most `CHUNK_SIZE_LIMIT` characters long,
/// with an overlap of `CHUNK_OVERLAP` characters.
fn split_long_text(text: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut start = 0;

    while start < chars.len() {
        let end = std::cmp::min(start + CHUNK_SIZE_LIMIT, chars.len());
        let chunk: String = chars[start..end].iter().collect();
        chunks.push(chunk);

        // Move the start for the next chunk, considering the overlap.
        // If we're at the end, or if the next step would not advance us, break the loop.
        let next_start = start + CHUNK_SIZE_LIMIT - CHUNK_OVERLAP;
        if next_start >= chars.len() || next_start <= start {
            break;
        }
        start = next_start;
    }

    chunks
}
