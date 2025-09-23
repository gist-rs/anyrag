//! # `anyrag-markdown`: Markdown File Ingestion Plugin
//!
//! This crate provides the logic for ingesting local Markdown files as a self-contained
//! plugin for the `anyrag` ecosystem. It implements the `Ingestor` trait from the
//! core `anyrag` library.

use anyhow::anyhow;
use anyrag::ingest::{IngestError as AnyragIngestError, IngestionResult, Ingestor};
use anyrag::{
    providers::{ai::generate_embeddings_batch, db::sqlite::SqliteProvider},
    PromptError,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;
use turso::params;
use uuid::Uuid;

// --- Error Definitions ---

#[derive(Error, Debug)]
pub enum MarkdownIngestError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Database error: {0}")]
    Database(#[from] turso::Error),
    #[error("Provider setup failed: {0}")]
    Provider(#[from] PromptError),
    #[error("Embedding generation failed: {0}")]
    Embedding(PromptError),
    #[error("Source deserialization failed: {0}")]
    SourceDeserialization(#[from] serde_json::Error),
}

impl From<MarkdownIngestError> for AnyragIngestError {
    fn from(err: MarkdownIngestError) -> Self {
        match err {
            MarkdownIngestError::Database(e) => AnyragIngestError::Database(e),
            MarkdownIngestError::Io(e) => AnyragIngestError::Fetch(e.to_string()),
            MarkdownIngestError::SourceDeserialization(e) => {
                AnyragIngestError::Parse(e.to_string())
            }
            _ => AnyragIngestError::Internal(anyhow!(err.to_string())),
        }
    }
}

// --- Data Structures ---

#[derive(Deserialize, Serialize, Debug)]
pub struct EmbeddingConfig {
    pub api_url: String,
    pub model: String,
    pub api_key: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct MarkdownSource {
    pub db_path: String,
    pub file_path: String,
    pub separator: String,
    pub embedding_config: Option<EmbeddingConfig>,
}

// --- Ingestor Implementation ---

pub struct MarkdownIngestor;

#[async_trait]
impl Ingestor for MarkdownIngestor {
    /// Ingests a Markdown file.
    ///
    /// The `source` argument is expected to be a JSON string matching the `MarkdownSource` struct.
    async fn ingest(
        &self,
        source: &str,
        owner_id: Option<&str>,
    ) -> Result<IngestionResult, AnyragIngestError> {
        let source_payload: MarkdownSource =
            serde_json::from_str(source).map_err(MarkdownIngestError::from)?;

        let file_path = &source_payload.file_path;
        let db_path = &source_payload.db_path;

        info!("Ingesting markdown file '{file_path}' into database '{db_path}'");
        let content = std::fs::read_to_string(file_path).map_err(MarkdownIngestError::from)?;
        let chunks: Vec<String> = content
            .split(&source_payload.separator)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if chunks.is_empty() {
            info!("No non-empty chunks found in '{file_path}'.");
            return Ok(Default::default());
        }
        info!("Found {} non-empty chunks to ingest.", chunks.len());

        let provider = SqliteProvider::new(db_path)
            .await
            .map_err(MarkdownIngestError::from)?;
        provider
            .initialize_schema()
            .await
            .map_err(MarkdownIngestError::from)?;
        let mut conn = provider.db.connect()?;

        // --- Ingest Chunks ---
        let tx = conn.transaction().await?;
        let mut ingested_ids = Vec::new();

        for (i, chunk) in chunks.iter().enumerate() {
            let document_id = Uuid::new_v4().to_string();
            let source_url = format!("{file_path}#chunk_{i}");
            let title: String = chunk.chars().take(80).collect();

            tx.execute(
                "INSERT INTO documents (id, owner_id, source_url, title, content)
                 VALUES (?, ?, ?, ?, ?)
                 ON CONFLICT(source_url) DO UPDATE SET
                 title = excluded.title,
                 content = excluded.content",
                params![
                    document_id.clone(),
                    owner_id,
                    source_url,
                    title,
                    chunk.clone()
                ],
            )
            .await?;
            ingested_ids.push(document_id);
        }
        tx.commit().await?;

        let documents_added = ingested_ids.len();

        // --- Embedding Generation ---
        if let Some(config) = source_payload.embedding_config {
            if !ingested_ids.is_empty() {
                println!(
                    "Generating embeddings for {} new chunks",
                    ingested_ids.len()
                );
                info!(
                    "Generating embeddings for {} new chunks using model '{}'...",
                    ingested_ids.len(),
                    config.model
                );
                let texts_to_embed: Vec<&str> = chunks.iter().map(AsRef::as_ref).collect();

                let embeddings = generate_embeddings_batch(
                    &config.api_url,
                    &config.model,
                    &texts_to_embed,
                    config.api_key.as_deref(),
                )
                .await
                .map_err(MarkdownIngestError::Embedding)?;

                let mut embedded_count = 0;
                for (doc_id, vector) in ingested_ids.iter().zip(embeddings) {
                    let vector_bytes: &[u8] = unsafe {
                        std::slice::from_raw_parts(vector.as_ptr() as *const u8, vector.len() * 4)
                    };

                    conn.execute(
                        "INSERT INTO document_embeddings (document_id, model_name, embedding) VALUES (?, ?, ?)",
                        params![doc_id.clone(), config.model.to_string(), vector_bytes],
                    )
                    .await?;
                    embedded_count += 1;
                }
                info!("Successfully generated and stored embeddings for {embedded_count} chunks.");
            }
        }

        Ok(IngestionResult {
            documents_added,
            source: file_path.to_string(),
            document_ids: ingested_ids,
            metadata: None,
        })
    }
}
