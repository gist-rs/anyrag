//! # anyrag-github: GitHub Ingestion and Search Crate
//!
//! This crate contains all functionality related to ingesting code examples
//! from public GitHub repositories and searching them for Retrieval-Augmented
//! Generation (RAG).

pub mod cli;
pub mod ingest;

// Re-export the main functions for easy access from other crates.
pub use ingest::{run_github_ingestion, search_examples, types};

use crate::ingest::{storage::StorageManager, types::IngestionTask};
use anyrag::ingest::{IngestError, IngestionResult, Ingestor};
use async_trait::async_trait;
use types::GitHubIngestError;

impl From<GitHubIngestError> for IngestError {
    fn from(err: GitHubIngestError) -> Self {
        match err {
            GitHubIngestError::Database(e) => IngestError::Database(e),
            GitHubIngestError::VersionNotFound(s) => IngestError::SourceNotFound(s),
            e => IngestError::Internal(anyhow::anyhow!(e.to_string())),
        }
    }
}

/// The Ingestor implementation for public GitHub repositories.
pub struct GithubIngestor;

#[async_trait]
impl Ingestor for GithubIngestor {
    async fn ingest(
        &self,
        source: &str, // The URL of the repository
        _owner_id: Option<&str>,
    ) -> Result<IngestionResult, IngestError> {
        // For GitHub ingestion, the server/caller needs to provide more configuration
        // than just the URL (e.g., version, embedding details).
        // This implementation will rely on environment variables for now, assuming
        // they are set by the execution context (like the server's state).
        // A more robust solution might involve a typed `source` parameter.
        let task = IngestionTask {
            url: source.to_string(),
            version: None, // The server handler will need to be adapted to provide this if needed.
            embedding_api_url: std::env::var("EMBEDDINGS_API_URL").ok(),
            embedding_model: std::env::var("EMBEDDINGS_MODEL").ok(),
            embedding_api_key: std::env::var("AI_API_KEY").ok(),
        };

        let storage_manager = StorageManager::new("db/github_ingest").await?;
        let (ingested_count, ingested_version) =
            run_github_ingestion(&storage_manager, task).await?;

        Ok(IngestionResult {
            source: format!("{source}#{ingested_version}"),
            documents_added: ingested_count,
            document_ids: vec![], // The current function doesn't return IDs. This can be added later.
        })
    }
}
