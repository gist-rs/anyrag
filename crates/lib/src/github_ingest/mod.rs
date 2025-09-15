//! # GitHub Repository Ingestion
//!
//! This module contains the complete pipeline for crawling a GitHub repository,
//! extracting versioned code examples, and storing them in a structured format
//! for Retrieval-Augmented Generation (RAG).

pub mod crawler;
pub mod extractor;
pub mod search_logic;
pub mod storage;
pub mod types;

use self::{
    crawler::Crawler,
    extractor::Extractor,
    search_logic::search_across_repos,
    storage::StorageManager,
    types::{GitHubIngestError, IngestionTask},
};
use crate::{providers::ai::AiProvider, SearchResult};
use std::sync::Arc;
use tracing::{info, instrument};

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
/// A tuple containing the number of examples ingested and the actual version string used.
#[instrument(skip(storage_manager, task), fields(url = %task.url, version = ?task.version))]
pub async fn run_github_ingestion(
    storage_manager: &StorageManager,
    task: IngestionTask,
) -> Result<(usize, String), GitHubIngestError> {
    info!("Starting GitHub ingestion pipeline.");

    // 1. Setup
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

    // 5. Embed new examples if embedding is configured.
    if let (Some(url), Some(model)) = (&task.embedding_api_url, &task.embedding_model) {
        // We only run embedding if new examples were actually stored.
        if count > 0 {
            info!("Starting embedding process for {} new examples.", count);
            storage_manager
                .embed_and_store_examples(
                    &tracked_repo,
                    url,
                    model,
                    task.embedding_api_key.as_deref(),
                )
                .await?;
        }
    }

    info!(
        "GitHub ingestion pipeline finished successfully. Ingested {} examples.",
        count
    );
    Ok((count, crawl_result.version))
}

/// Searches for examples across multiple repositories.
pub async fn search_examples(
    storage_manager: &StorageManager,
    query: &str,
    repos: &[String],
    ai_provider: Arc<dyn AiProvider>,
    embedding_api_url: &str,
    embedding_model: &str,
    embedding_api_key: Option<&str>,
) -> Result<Vec<SearchResult>, GitHubIngestError> {
    search_across_repos(
        query,
        repos,
        storage_manager,
        ai_provider,
        embedding_api_url,
        embedding_model,
        embedding_api_key,
    )
    .await
}
