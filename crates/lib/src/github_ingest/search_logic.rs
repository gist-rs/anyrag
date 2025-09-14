//! # Multi-Repository Search Logic for GitHub Ingestion
//!
//! This module provides the functionality to search for code examples across
//! multiple, isolated repository-specific databases, implementing the logic
//! for the RAG query engine.

use super::{storage::StorageManager, types::GitHubIngestError};
use crate::{
    ingest::knowledge::clean_llm_response,
    prompts::knowledge::{
        GITHUB_EXAMPLE_SEARCH_ANALYSIS_SYSTEM_PROMPT, GITHUB_EXAMPLE_SEARCH_ANALYSIS_USER_PROMPT,
    },
    providers::ai::{generate_embedding, AiProvider},
    rerank::reciprocal_rank_fusion,
    PromptError, SearchResult,
};
use futures::future::join_all;
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, info, warn};
use turso::{params, Value as TursoValue};

/// Parses a repository specification string (e.g., "tursodatabase-turso:v1.0.0")
/// into a repository name and an optional version.
fn parse_repo_spec(repo_spec: &str) -> (String, Option<String>) {
    if let Some((name, version)) = repo_spec.rsplit_once(':') {
        if !version.is_empty() {
            return (name.to_string(), Some(version.to_string()));
        }
    }
    (repo_spec.to_string(), None)
}

/// Performs a keyword search within a single repository's database.
async fn keyword_search_for_repo(
    storage_manager: &StorageManager,
    repo_name: &str,
    version: &str,
    query: &str,
    limit: u32,
) -> Result<Vec<SearchResult>, GitHubIngestError> {
    let provider = storage_manager.get_provider_for_repo(repo_name).await?;
    let conn = provider.db.connect()?;

    let pattern = format!("%{query}%");
    let sql = format!(
        "
        SELECT example_handle, source_file, content
        FROM generated_examples
        WHERE version = ? AND (content LIKE ? OR example_handle LIKE ?)
        LIMIT {limit}
    "
    );

    let mut rows = conn
        .query(&sql, params![version.to_string(), pattern.clone(), pattern])
        .await?;

    let mut results = Vec::new();
    while let Some(row) = rows.next().await? {
        results.push(SearchResult {
            title: row.get(0)?,
            link: row.get(1)?,
            description: row.get(2)?,
            score: 0.5, // Default score for keyword search
        });
    }
    Ok(results)
}

/// Performs a vector similarity search within a single repository's database.
async fn vector_search_for_repo(
    storage_manager: &StorageManager,
    repo_name: &str,
    version: &str,
    query_vector: &[f32],
    limit: u32,
) -> Result<Vec<SearchResult>, GitHubIngestError> {
    let provider = storage_manager.get_provider_for_repo(repo_name).await?;
    let conn = provider.db.connect()?;

    let vector_str = format!(
        "vector('[{}]')",
        query_vector
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let distance_calc = format!("(1.0 - (vector_distance_cos(ee.embedding, {vector_str}) / 2.0))");

    let sql = format!(
        "
        SELECT ge.example_handle, ge.source_file, ge.content, {distance_calc} AS similarity
        FROM example_embeddings ee
        JOIN generated_examples ge ON ee.example_id = ge.id
        WHERE ge.version = ? AND ee.embedding IS NOT NULL
        ORDER BY similarity DESC
        LIMIT {limit}
    "
    );

    let mut rows = conn.query(&sql, params![version.to_string()]).await?;

    let mut results = Vec::new();
    while let Some(row) = rows.next().await? {
        results.push(SearchResult {
            title: row.get(0)?,
            link: row.get(1)?,
            description: row.get(2)?,
            score: match row.get_value(3)? {
                TursoValue::Real(f) => f,
                _ => 0.0,
            },
        });
    }
    Ok(results)
}

#[derive(Deserialize, Debug)]
struct AnalyzedQuery {
    #[serde(default)]
    #[allow(dead_code)] // TODO: remove this
    entities: Vec<String>,
    #[serde(default)]
    keyphrases: Vec<String>,
}

/// Uses an LLM to extract entities and keyphrases from a user query for code search.
async fn analyze_query(
    ai_provider: &dyn AiProvider,
    query_text: &str,
    system_prompt: &str,
    user_prompt_template: &str,
) -> Result<AnalyzedQuery, PromptError> {
    let user_prompt = user_prompt_template.replace("{prompt}", query_text);
    let llm_response = ai_provider.generate(system_prompt, &user_prompt).await?;

    debug!("LLM query analysis response: {}", llm_response);
    let cleaned_response = clean_llm_response(&llm_response);

    match serde_json::from_str(cleaned_response) {
        Ok(parsed) => Ok(parsed),
        Err(e) => {
            warn!(
                "Failed to parse query analysis JSON, falling back to using full query as keyphrase. Error: {}. Raw response: '{}'",
                e, cleaned_response
            );
            // Fallback: use the original query as a keyphrase
            Ok(AnalyzedQuery {
                entities: Vec::new(),
                keyphrases: vec![query_text.to_string()],
            })
        }
    }
}

/// The main entry point for searching across multiple repositories.
pub async fn search_across_repos(
    query: &str,
    repos: &[String],
    storage_manager: &StorageManager,
    ai_provider: Arc<dyn AiProvider>,
    embedding_api_url: &str,
    embedding_model: &str,
) -> Result<Vec<SearchResult>, GitHubIngestError> {
    info!(
        "Starting multi-repo search for query: '{}' in repos: {:?}",
        query, repos
    );

    // 1. Analyze the user's query to extract key terms.
    let analyzed_query = analyze_query(
        ai_provider.as_ref(),
        query,
        GITHUB_EXAMPLE_SEARCH_ANALYSIS_SYSTEM_PROMPT,
        GITHUB_EXAMPLE_SEARCH_ANALYSIS_USER_PROMPT,
    )
    .await
    .map_err(GitHubIngestError::Prompt)?;

    let keyword_query = analyzed_query.keyphrases.join(" ");

    // 2. Generate an embedding for the original, full query for vector search.
    let query_vector = generate_embedding(embedding_api_url, embedding_model, query)
        .await
        .map_err(|e| GitHubIngestError::Internal(e.into()))?;

    let mut search_handles = vec![];

    for repo_spec in repos {
        let (repo_name, version_opt) = parse_repo_spec(repo_spec);
        // Use the refined keyword query for keyword search, but the original for vector search.
        let query_clone = if keyword_query.trim().is_empty() {
            query.to_string()
        } else {
            keyword_query.clone()
        };
        let query_vector_clone = query_vector.clone();
        let storage_manager_clone = storage_manager.clone();

        let handle: tokio::task::JoinHandle<Result<Vec<SearchResult>, GitHubIngestError>> =
            tokio::spawn(async move {
                let version = match version_opt {
                    Some(v) => v,
                    None => storage_manager_clone
                        .get_latest_version(&repo_name)
                        .await?
                        .ok_or_else(|| {
                            GitHubIngestError::VersionNotFound(format!(
                                "No versions found for repo '{repo_name}'"
                            ))
                        })?,
                };

                let (keyword_res, vector_res) = tokio::join!(
                    keyword_search_for_repo(
                        &storage_manager_clone,
                        &repo_name,
                        &version,
                        &query_clone,
                        20
                    ),
                    vector_search_for_repo(
                        &storage_manager_clone,
                        &repo_name,
                        &version,
                        &query_vector_clone,
                        20
                    )
                );

                // Combine and return results for this repo, handling errors
                let keyword_results = match keyword_res {
                    Ok(res) => res,
                    Err(e) => {
                        warn!(
                            "Keyword search failed for repo '{}': {}",
                            &repo_name,
                            e.to_string()
                        );
                        vec![]
                    }
                };

                let vector_results = match vector_res {
                    Ok(res) => res,
                    Err(e) => {
                        warn!(
                            "Vector search failed for repo '{}': {}",
                            &repo_name,
                            e.to_string()
                        );
                        vec![]
                    }
                };

                Ok(reciprocal_rank_fusion(vector_results, keyword_results))
            });
        search_handles.push(handle);
    }

    let all_repo_results = join_all(search_handles).await;

    let mut combined_results = vec![];
    for result in all_repo_results {
        match result {
            Ok(Ok(repo_results)) => combined_results.extend(repo_results),
            Ok(Err(e)) => warn!("A repo search task failed: {}", e),
            Err(e) => warn!("A repo search task panicked: {}", e),
        }
    }

    // A final re-ranking across all repositories to get the best overall results
    combined_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    combined_results.truncate(20);

    Ok(combined_results)
}
