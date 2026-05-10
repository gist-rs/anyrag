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
use anyrag::{providers::ai::AiProvider, SearchResult};
use glob::Pattern;
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

    // 1b. Cache check — skip clone if version already ingested (unless force)
    if !task.force {
        let version_to_check = task.version.as_deref().unwrap_or("");
        if !version_to_check.is_empty() {
            let exists = storage_manager
                .version_exists(&tracked_repo.repo_name, version_to_check)
                .await?;
            if exists {
                // Retrieve cached count
                let cached_examples = storage_manager
                    .get_examples(&tracked_repo.repo_name, version_to_check)
                    .await?;
                let cached_count = cached_examples.len();
                info!(
                    cached_count,
                    "Version '{version_to_check}' already ingested. Skipping clone (use --force to override)."
                );
                return Ok((cached_count, version_to_check.to_string()));
            }
        }
    }

    // 2. Crawl
    let crawl_result = Crawler::crawl(&task).await?;

    // 2b. Second cache check with discovered version (for tasks without explicit version)
    if !task.force {
        let exists = storage_manager
            .version_exists(&tracked_repo.repo_name, &crawl_result.version)
            .await?;
        if exists {
            let cached_examples = storage_manager
                .get_examples(&tracked_repo.repo_name, &crawl_result.version)
                .await?;
            let cached_count = cached_examples.len();
            info!(
                cached_count,
                "Version '{}' already ingested. Skipping extraction (use --force to override).",
                crawl_result.version
            );
            return Ok((cached_count, crawl_result.version));
        }
    }

    // 3. Compile exclude patterns from task
    let compiled_excludes: Vec<Pattern> = task
        .excludes
        .as_ref()
        .map(|excludes| {
            excludes
                .iter()
                .filter_map(|s| match Pattern::new(s) {
                    Ok(p) => Some(p),
                    Err(e) => {
                        info!("Invalid exclude glob pattern '{}': {}", s, e);
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    // 4. Extract based on dump_type
    let examples = match task.dump_type {
        types::DumpType::Examples => Extractor::extract(
            &crawl_result.path,
            &crawl_result.version,
            task.extract_included_files,
            &task.includes,
            &compiled_excludes,
        )?,
        types::DumpType::Tests => Extractor::extract_all_tests(
            &crawl_result.path,
            &crawl_result.version,
            &task.includes,
            &compiled_excludes,
        )?,
        types::DumpType::Src => {
            // Src dump type doesn't extract examples - handled separately in CLI
            info!("Src dump type selected - skipping example extraction.");
            vec![]
        }
    };

    // 5. Store
    let count = storage_manager
        .store_examples(&tracked_repo, examples)
        .await?;

    // 6. Embed new examples if embedding is configured.
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
