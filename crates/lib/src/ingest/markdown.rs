//! # Markdown File Ingestion
//!
//! This module provides the logic for reading a local Markdown file,
//! splitting it into chunks by a separator, and ingesting those chunks
//! into the database as individual documents.

use crate::{
    ingest::text::ingest_chunks_as_documents,
    providers::{ai::generate_embedding, db::sqlite::SqliteProvider},
    PromptError,
};
use thiserror::Error;
use tracing::info;
use turso::params;

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
    #[error("Embedding generation failed: {0}")]
    Embedding(PromptError),
}

/// Configuration for generating embeddings during ingestion.
pub struct EmbeddingConfig<'a> {
    pub api_url: &'a str,
    pub model: &'a str,
    pub api_key: Option<&'a str>,
}

/// Reads a Markdown file, splits it into chunks by a separator, and ingests them.
///
/// This function is public and can be called from other parts of the application,
/// such as the CLI.
pub async fn ingest_markdown_file(
    db_path: &str,
    file_path: &str,
    separator: &str,
    embedding_config: Option<EmbeddingConfig<'_>>,
) -> Result<usize, MarkdownIngestError> {
    info!("Ingesting markdown file '{file_path}' into database '{db_path}'");
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

    let ingested_ids =
        ingest_chunks_as_documents(&mut conn, chunks.clone(), file_path, None).await?;

    // --- Embedding Generation ---
    if let Some(config) = embedding_config {
        if !ingested_ids.is_empty() {
            println!(
                "Generating embeddings for {} new chunks using model '{}'...",
                ingested_ids.len(),
                config.model
            );
            let mut embedded_count = 0;
            // Pair the returned IDs with their original content. The order is preserved.
            let id_chunk_pairs: Vec<_> = ingested_ids
                .iter()
                .cloned()
                .zip(chunks.into_iter())
                .collect();

            for (doc_id, chunk_content) in id_chunk_pairs {
                let vector = generate_embedding(
                    config.api_url,
                    config.model,
                    &chunk_content,
                    config.api_key,
                )
                .await
                .map_err(MarkdownIngestError::Embedding)?;

                // Convert Vec<f32> to &[u8] for BLOB storage
                let vector_bytes: &[u8] = unsafe {
                    std::slice::from_raw_parts(vector.as_ptr() as *const u8, vector.len() * 4)
                };

                conn.execute(
                    "INSERT INTO document_embeddings (document_id, model_name, embedding) VALUES (?, ?, ?)",
                    params![doc_id, config.model.to_string(), vector_bytes],
                )
                .await?;
                embedded_count += 1;
            }
            println!(
                "✅ Successfully generated and stored embeddings for {embedded_count} chunks."
            );
        }
    }

    Ok(ingested_ids.len())
}
