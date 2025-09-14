//! # GitHub Repository Ingestion
//!
//! This module contains the complete pipeline for crawling a GitHub repository,
//! extracting versioned code examples, and storing them in a structured format
//! for Retrieval-Augmented Generation (RAG).

pub mod crawler;
pub mod extractor;
pub mod storage;
pub mod types;

use self::{
    crawler::Crawler,
    extractor::Extractor,
    storage::StorageManager,
    types::{GitHubIngestError, IngestionTask},
};
use crate::SearchResult;
use tracing::{info, instrument};

const DEFAULT_DB_DIR: &str = "db/github_ingest";

/// The main orchestrator for the GitHub ingestion pipeline.
///
/// This function takes an `IngestionTask` and performs the following steps:
/// 1. Initializes the `StorageManager`.
/// 2. Tracks the repository to get its dedicated database path.
/// 3. Crawls the repository, cloning it into a temporary directory.
/// 4. Extracts all code examples from the cloned repository.
/// 5. Stores the extracted examples in the database.
///
/// # Arguments
/// * `task`: The `IngestionTask` specifying the repository URL and version.
///
/// # Returns
/// The number of examples that were successfully ingested.
#[instrument(skip(task), fields(url = %task.url, version = ?task.version))]
pub async fn run_github_ingestion(task: IngestionTask) -> Result<usize, GitHubIngestError> {
    info!("Starting GitHub ingestion pipeline.");

    // 1. Setup
    let storage_manager = StorageManager::new(DEFAULT_DB_DIR).await?;
    let tracked_repo = storage_manager.track_repository(&task.url).await?;

    // 2. Crawl
    let crawl_result = Crawler::crawl(&task).await?;

    // TODO: Add logic to determine the latest version if none is specified in the task.
    // For now, the version returned by crawl() is used.

    // 3. Extract
    let examples = Extractor::extract(&crawl_result.path, &crawl_result.version)?;

    // 4. Store
    let count = storage_manager
        .store_examples(&tracked_repo, examples)
        .await?;

    info!(
        "GitHub ingestion pipeline finished successfully. Ingested {} examples.",
        count
    );
    Ok(count)
}

/// Searches for examples across multiple repositories.
pub async fn search_examples(
    _query: &str,
    _repos: &[String],
) -> Result<Vec<SearchResult>, GitHubIngestError> {
    info!("Searching examples...");
    // Placeholder implementation
    Ok(vec![SearchResult {
        title: "Placeholder".to_string(),
        link: "placeholder".to_string(),
        description: "This is a placeholder result.".to_string(),
        score: 1.0,
    }])
}
