//! # Search Logic
//!
//! This module provides the core logic for all types of search:
//! - Vector (semantic) search for finding conceptually similar articles.
//! - Keyword (lexical) search for finding exact term matches.
//! - Hybrid search for combining the strengths of both using either an LLM for
//!   re-ranking (default) or Reciprocal Rank Fusion (optional).

use crate::{
    providers::{
        ai::AiProvider,
        db::storage::{KeywordSearch, VectorSearch},
    },
    rerank::{llm_rerank, reciprocal_rank_fusion, RerankError},
    types::SearchResult,
};
use serde::Deserialize;
use std::collections::HashMap;
use thiserror::Error;
use tracing::info;

/// Defines the re-ranking strategy for hybrid search.
#[derive(Default, Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    /// Uses a Large Language Model to intelligently re-rank candidates. (Default)
    #[default]
    LlmReRank,
    /// Uses the fast Reciprocal Rank Fusion algorithm.
    Rrf,
}

/// Custom error types for the search process.
#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Database error: {0}")]
    Database(#[from] turso::Error),
    #[error("Re-ranking failed: {0}")]
    ReRank(#[from] RerankError),
}

/// Performs a hybrid search by fetching candidates and then using a specified
/// `SearchMode` to re-rank them.
pub async fn hybrid_search<P>(
    provider: &P,
    ai_provider: &dyn AiProvider,
    query_vector: Vec<f32>,
    query_text: &str,
    limit: u32,
    mode: SearchMode,
) -> Result<Vec<SearchResult>, SearchError>
where
    P: VectorSearch + KeywordSearch + ?Sized,
{
    info!(
        "Starting hybrid search for: '{}' with mode: {:?}",
        query_text, mode
    );

    // --- Stage 1: Fetch Candidates Concurrently ---
    // We fetch more candidates than requested (limit * 2) to provide a richer
    // set of documents to the re-ranking algorithm, improving its ability to find
    // the most relevant results.
    let (vector_results, keyword_results) = tokio::join!(
        provider.vector_search(query_vector.clone(), limit * 2),
        provider.keyword_search(query_text, limit * 2)
    );

    let vector_results = vector_results?;
    info!("Vector search found {} candidates.", vector_results.len());
    let keyword_results = keyword_results?;
    info!("Keyword search found {} candidates.", keyword_results.len());

    // --- Stage 2: Re-rank using the specified mode ---
    let mut ranked_results = match mode {
        SearchMode::LlmReRank => {
            // Combine and deduplicate candidates from both search methods.
            let mut all_candidates: HashMap<String, SearchResult> = HashMap::new();
            for result in vector_results
                .into_iter()
                .chain(keyword_results.into_iter())
            {
                all_candidates.entry(result.link.clone()).or_insert(result);
            }
            let candidates: Vec<SearchResult> = all_candidates.into_values().collect();

            if candidates.is_empty() {
                return Ok(vec![]);
            }
            // The `?` operator here will convert a RerankError into a SearchError::Rerank
            llm_rerank(ai_provider, query_text, candidates).await?
        }
        SearchMode::Rrf => reciprocal_rank_fusion(vector_results, keyword_results),
    };

    ranked_results.truncate(limit as usize);
    Ok(ranked_results)
}
