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
use serde::Deserialize;
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

/// A struct to deserialize the `source` parameter for the `ingest` method.
#[derive(Deserialize)]
struct IngestSource {
    url: String,
    version: Option<String>,
    #[serde(default)]
    extract_included_files: bool,
}

use std::sync::Arc;

/// The Ingestor implementation for public GitHub repositories.
pub struct GithubIngestor {
    storage_manager: Arc<StorageManager>,
    embedding_api_url: Option<String>,
    embedding_model: Option<String>,
    embedding_api_key: Option<String>,
}

impl GithubIngestor {
    /// Creates a new, configurable `GithubIngestor`.
    /// This allows dependencies like embedding configuration to be injected by the caller
    /// (e.g., the server) instead of being read from environment variables.
    pub fn new(
        storage_manager: Arc<StorageManager>,
        embedding_api_url: Option<String>,
        embedding_model: Option<String>,
        embedding_api_key: Option<String>,
    ) -> Self {
        Self {
            storage_manager,
            embedding_api_url,
            embedding_model,
            embedding_api_key,
        }
    }
}

#[async_trait]
impl Ingestor for GithubIngestor {
    /// Ingests a GitHub repository.
    ///
    /// # Arguments
    /// * `source`: A JSON string containing the `url` and optional `version`.
    ///   Example: `{"url": "https://github.com/user/repo", "version": "v1.0.0"}`
    /// * `_owner_id`: The owner ID (not used in this implementation).
    async fn ingest(
        &self,
        source: &str,
        _owner_id: Option<&str>,
    ) -> Result<IngestionResult, IngestError> {
        // 1. Deserialize the source JSON to get the URL and version.
        let ingest_source: IngestSource = serde_json::from_str(source).map_err(|e| {
            IngestError::Parse(format!("Invalid source JSON for GitHub ingest: {e}"))
        })?;

        // 2. Create the IngestionTask using the configuration from the struct fields.
        let task = IngestionTask {
            url: ingest_source.url.clone(),
            version: ingest_source.version,
            embedding_api_url: self.embedding_api_url.clone(),
            embedding_model: self.embedding_model.clone(),
            embedding_api_key: self.embedding_api_key.clone(),
            extract_included_files: ingest_source.extract_included_files,
        };

        // 3. Run the ingestion pipeline.
        let (ingested_count, ingested_version) =
            run_github_ingestion(&self.storage_manager, task).await?;

        // 4. Return the standardized result.
        Ok(IngestionResult {
            source: format!("{}#{}", ingest_source.url, ingested_version),
            documents_added: ingested_count,
            document_ids: vec![], // The current function doesn't return IDs. This can be added later.
            metadata: None,
        })
    }
}
