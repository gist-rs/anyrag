//! # Markdown File Ingestion
//!
//! This module provides the logic for reading a local Markdown file,
//! splitting it into chunks by a separator, and ingesting those chunks
//! into the database as individual documents.

use crate::{
    ingest::text::ingest_chunks_as_documents, providers::db::sqlite::SqliteProvider, PromptError,
};
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum MarkdownIngestError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Database error: {0}")]
    Database(#[from] turso::Error),
    #[error("Ingestion error: {0}")]
    Ingest(#[from] crate::ingest::text::IngestError),
    #[error("Provider setup failed: {0}")]
    Provider(#[from] PromptError),
}

/// Reads a Markdown file, splits it into chunks by a separator, and ingests them.
///
/// This function is public and can be called from other parts of the application,
/// such as the CLI.
pub async fn ingest_markdown_file(
    db_path: &str,
    file_path: &str,
    separator: &str,
) -> Result<usize, MarkdownIngestError> {
    info!(
        "Ingesting markdown file '{}' into database '{}'",
        file_path, db_path
    );
    let content = std::fs::read_to_string(file_path)?;
    let chunks: Vec<String> = content
        .split(separator)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if chunks.is_empty() {
        println!("No non-empty chunks found in the file to ingest.");
        return Ok(0);
    }
    println!("Found {} non-empty chunks to ingest.", chunks.len());

    let provider = SqliteProvider::new(db_path).await?;
    provider.initialize_schema().await?;
    let mut conn = provider.db.connect()?;

    let ingested_ids = ingest_chunks_as_documents(&mut conn, chunks, file_path, None).await?;

    Ok(ingested_ids.len())
}
