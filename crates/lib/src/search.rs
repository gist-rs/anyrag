//! # Search Logic
//!
//! This module provides the core logic for all types of search:
//! - Vector (semantic) search for finding conceptually similar articles.
//! - Keyword (lexical) search for finding exact term matches.
//! - Hybrid search for combining the strengths of both using either an LLM for
//!   re-ranking (default) or Reciprocal Rank Fusion (optional).

use crate::{
    errors::PromptError,
    providers::ai::AiProvider,
    rerank::{llm_rerank, reciprocal_rank_fusion},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::info;
use turso::{params, Database, Value};

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
    #[error("LLM Re-ranking failed: {0}")]
    LlmReRank(#[from] PromptError),
    #[error("Failed to parse LLM re-ranking response: {0}")]
    LlmResponseParsing(#[from] serde_json::Error),
}

/// Represents a single search result.
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct SearchResult {
    pub title: String,
    pub link: String,
    pub description: String,
    /// A generic score. For vector search, lower is better (distance). For RRF/LLM, higher is better.
    pub score: f64,
}

/// Performs a vector-based (semantic) search for articles.
pub async fn search_by_vector(
    db: &Database,
    query_vector: Vec<f32>,
    limit: u32,
) -> Result<Vec<SearchResult>, SearchError> {
    let conn = db.connect().map_err(SearchError::Database)?;

    let vector_str = format!(
        "vector('[{}]')",
        query_vector
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let distance_threshold = 0.6;
    let sql = format!(
        "SELECT title, link, description, vector_distance_cos(embedding, {vector_str}) AS distance
         FROM articles
         WHERE embedding IS NOT NULL AND distance < {distance_threshold}
         ORDER BY distance ASC
         LIMIT {limit};"
    );

    info!("Executing vector search query.");
    let mut results = conn.query(&sql, ()).await?;
    let mut search_results = Vec::new();

    while let Some(row) = results.next().await? {
        let title = match row.get_value(0)? {
            Value::Text(s) => s,
            _ => String::new(),
        };
        let link = match row.get_value(1)? {
            Value::Text(s) => s,
            _ => String::new(),
        };
        let description = match row.get_value(2)? {
            Value::Text(s) => s,
            _ => String::new(),
        };
        let score = match row.get_value(3)? {
            Value::Real(f) => f,
            _ => 0.0,
        };
        search_results.push(SearchResult {
            title,
            link,
            description,
            score,
        });
    }

    Ok(search_results)
}

/// Performs a case-insensitive keyword-based search using SQL LIKE.
pub async fn search_by_keyword(
    db: &Database,
    query: &str,
    limit: u32,
) -> Result<Vec<SearchResult>, SearchError> {
    let conn = db.connect().map_err(SearchError::Database)?;
    // Convert the query to lowercase for a case-insensitive search.
    let pattern = format!("%{}%", query.to_lowercase());

    let sql = format!(
        "
        SELECT title, link, description, 0.0 as score
        FROM articles
        WHERE LOWER(title) LIKE ?1 OR LOWER(description) LIKE ?1
        LIMIT {limit};
    "
    );

    info!("Executing LIKE keyword search query for: {}", query);
    let mut results = conn.query(&sql, params![pattern]).await?;
    let mut search_results = Vec::new();

    while let Some(row) = results.next().await? {
        let title = match row.get_value(0)? {
            Value::Text(s) => s,
            _ => String::new(),
        };
        let link = match row.get_value(1)? {
            Value::Text(s) => s,
            _ => String::new(),
        };
        let description = match row.get_value(2)? {
            Value::Text(s) => s,
            _ => String::new(),
        };
        let score = match row.get_value(3)? {
            Value::Real(f) => f,
            _ => 0.0,
        };
        search_results.push(SearchResult {
            title,
            link,
            description,
            score,
        });
    }

    Ok(search_results)
}

/// Performs a hybrid search by fetching candidates and then using a specified
/// `SearchMode` to re-rank them.
pub async fn hybrid_search(
    db: &Database,
    ai_provider: &dyn AiProvider,
    query_vector: Vec<f32>,
    query_text: &str,
    limit: u32,
    mode: SearchMode,
) -> Result<Vec<SearchResult>, SearchError> {
    info!(
        "Starting hybrid search for: '{}' with mode: {:?}",
        query_text, mode
    );

    // --- Stage 1: Fetch Candidates Concurrently ---
    let (vector_results, keyword_results) = tokio::join!(
        search_by_vector(db, query_vector.clone(), limit * 2),
        search_by_keyword(db, query_text, limit * 2)
    );

    let vector_results = vector_results?;
    info!("Vector search found {} candidates.", vector_results.len());
    let keyword_results = keyword_results?;
    info!("Keyword search found {} candidates.", keyword_results.len());

    // --- Stage 2: Re-rank using the specified mode ---
    let mut ranked_results = match mode {
        SearchMode::LlmReRank => {
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
            llm_rerank(ai_provider, query_text, candidates).await?
        }
        SearchMode::Rrf => reciprocal_rank_fusion(vector_results, keyword_results),
    };

    ranked_results.truncate(limit as usize);
    Ok(ranked_results)
}
