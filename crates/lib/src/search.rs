//! # Search Logic
//!
//! This module provides the core logic for all types of search:
//! - Vector (semantic) search for finding conceptually similar articles.
//! - Keyword (lexical) search for finding exact term matches.
//! - Hybrid search for combining the strengths of both using Reciprocal Rank Fusion.

use crate::providers::ai::AiProvider;
use serde::Serialize;
use std::collections::HashMap;
use thiserror::Error;
use tracing::info;
use turso::{params, Database, Value as TursoValue};

/// Custom error types for the search process.
#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Database error: {0}")]
    Database(#[from] turso::Error),
}

/// Represents a single search result from a vector similarity search.
#[derive(Debug, Serialize, Clone)]
pub struct SearchResult {
    pub title: String,
    pub link: String,
    pub description: String,
    /// A generic score. For vector search, lower is better (distance). For keyword/hybrid, higher is better.
    pub score: f64,
}

/// Performs a vector-based (semantic) search for articles.
///
/// # Arguments
///
/// * `db`: A shared reference to the Turso database instance.
/// * `query_vector`: The vector to compare against.
/// * `limit`: The maximum number of results to return.
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

    // The vector_distance_cos function returns a value between 0.0 (identical) and 2.0.
    // A value of 1.0 means the vectors are orthogonal (no similarity).
    // We add a WHERE clause to filter out results that are not semantically similar,
    // using a threshold. A value of 0.6 is a reasonable starting point, filtering
    // out anything that is more dissimilar than similar.
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
        let title: String = match row.get_value(0)? {
            TursoValue::Text(s) => s,
            _ => String::new(),
        };
        let link: String = match row.get_value(1)? {
            TursoValue::Text(s) => s,
            _ => String::new(),
        };
        let description: String = match row.get_value(2)? {
            TursoValue::Text(s) => s,
            _ => String::new(),
        };
        let score: f64 = match row.get_value(3)? {
            TursoValue::Real(f) => f,
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

/// Performs a keyword-based search using SQL LIKE.
///
/// # Arguments
///
/// * `db`: A shared reference to the Turso database instance.
/// * `query`: The text to search for.
/// * `limit`: The maximum number of results to return.
pub async fn search_by_keyword(
    db: &Database,
    query: &str,
    limit: u32,
) -> Result<Vec<SearchResult>, SearchError> {
    let conn = db.connect().map_err(SearchError::Database)?;
    let pattern = format!("%{query}%");

    let sql = format!(
        "
        SELECT title, link, description, 0.0 as score
        FROM articles
        WHERE title LIKE ?1 OR description LIKE ?1
        LIMIT {limit};
    "
    );

    info!("Executing LIKE keyword search query for: {}", query);
    let mut results = conn.query(&sql, params![pattern]).await?;
    let mut search_results = Vec::new();

    while let Some(row) = results.next().await? {
        let title = match row.get_value(0)? {
            TursoValue::Text(s) => s,
            _ => String::new(),
        };
        let link = match row.get_value(1)? {
            TursoValue::Text(s) => s,
            _ => String::new(),
        };
        let description = match row.get_value(2)? {
            TursoValue::Text(s) => s,
            _ => String::new(),
        };
        let score = match row.get_value(3)? {
            TursoValue::Real(f) => f,
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

/// Performs a hybrid search by combining vector and keyword search results
/// using Reciprocal Rank Fusion (RRF).
///
/// # Arguments
///
/// * `db`: A shared reference to the Turso database instance.
/// * `_ai_provider`: Unused in this RRF implementation, but kept for signature compatibility.
/// * `query_vector`: The vector for the semantic search part.
/// * `query_text`: The original user query text for keyword search.
/// * `limit`: The maximum number of final results to return.
pub async fn hybrid_search(
    db: &Database,
    _ai_provider: &dyn AiProvider,
    query_vector: Vec<f32>,
    query_text: &str,
    limit: u32,
) -> Result<Vec<SearchResult>, SearchError> {
    info!("Starting hybrid search for: '{}'", query_text);

    // --- Stage 1: Fetch Candidates Concurrently ---
    let (vector_results, keyword_results) = tokio::join!(
        search_by_vector(db, query_vector, limit * 2), // Fetch more to allow for diverse ranking
        search_by_keyword(db, query_text, limit * 2)
    );

    let vector_results = vector_results?;
    info!("Vector search results: {:?}", vector_results);
    let keyword_results = keyword_results?;
    info!("Keyword search results: {:?}", keyword_results);

    // --- Stage 2: Combine and Re-rank using Reciprocal Rank Fusion (RRF) ---
    let mut rrf_scores: HashMap<String, f64> = HashMap::new();
    let k = 60.0; // RRF ranking constant
    let keyword_boost = 1.2; // Give a slight boost to exact keyword matches

    // Process vector results
    for (i, result) in vector_results.iter().enumerate() {
        let rank = (i + 1) as f64;
        *rrf_scores.entry(result.link.clone()).or_insert(0.0) += 1.0 / (k + rank);
    }

    // Process keyword results with the boost
    for (i, result) in keyword_results.iter().enumerate() {
        let rank = (i + 1) as f64;
        let score = (1.0 / (k + rank)) * keyword_boost;
        *rrf_scores.entry(result.link.clone()).or_insert(0.0) += score;
    }

    info!("RRF scores: {:?}", rrf_scores);

    // --- Stage 3: Sort and Finalize Results ---
    let mut combined_results: Vec<SearchResult> = vector_results
        .into_iter()
        .chain(keyword_results.into_iter())
        .map(|res| (res.link.clone(), res))
        .collect::<HashMap<_, _>>()
        .into_values()
        .collect();

    combined_results.sort_by(|a, b| {
        let score_a = rrf_scores.get(&a.link).unwrap_or(&0.0);
        let score_b = rrf_scores.get(&b.link).unwrap_or(&0.0);
        score_b
            .partial_cmp(score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    info!("Sorted combined results: {:?}", combined_results);

    for result in &mut combined_results {
        result.score = *rrf_scores.get(&result.link).unwrap_or(&0.0);
    }

    combined_results.truncate(limit as usize);
    Ok(combined_results)
}
