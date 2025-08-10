//! # Search Logic
//!
//! This module provides the core logic for all types of search:
//! - Vector (semantic) search for finding conceptually similar articles.
//! - Keyword (lexical) search for finding exact term matches.
//! - Hybrid search for combining the strengths of both using Reciprocal Rank Fusion.
//!
//! It also contains the logic for generating and storing vector embeddings.

use crate::providers::ai::embedding::generate_embedding;
use crate::providers::ai::AiProvider;
use serde::Serialize;
use std::collections::HashMap;
use thiserror::Error;
use tracing::info;
use turso::{params, Database, Value as TursoValue};

/// Custom error types for the embedding and search processes.
#[derive(Error, Debug)]
pub enum EmbeddingError {
    #[error("Database error: {0}")]
    Database(#[from] turso::Error),
    #[error("Embedding generation failed: {0}")]
    Embedding(#[from] crate::errors::PromptError),
    #[error("Article with ID {0} not found.")]
    NotFound(i64),
    #[error("Failed to serialize vector for query.")]
    VectorSerialization,
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

/// Fetches an article, generates an embedding for it, and saves it to the database.
///
/// # Arguments
///
/// * `db`: A shared reference to the Turso database instance.
/// * `embeddings_api_url`: The URL of the embeddings API endpoint.
/// * `embeddings_model`: The name of the model to use for generating embeddings.
/// * `article_id`: The ID of the article to process.
pub async fn embed_article(
    db: &Database,
    embeddings_api_url: &str,
    embeddings_model: &str,
    article_id: i64,
) -> Result<(), EmbeddingError> {
    let conn = db.connect().map_err(EmbeddingError::Database)?;

    // 1. Fetch the article's text content.
    let mut stmt = conn
        .prepare("SELECT title, description FROM articles WHERE id = ?")
        .await?;
    let mut rows = stmt.query(params![article_id]).await?;

    let (title, description) = if let Some(row) = rows.next().await? {
        let title: String = match row.get_value(0)? {
            TursoValue::Text(s) => s,
            _ => String::new(),
        };
        let description: String = match row.get_value(1)? {
            TursoValue::Text(s) => s,
            _ => String::new(),
        };
        (title, description)
    } else {
        return Err(EmbeddingError::NotFound(article_id));
    };

    // Use both title and description for a richer embedding context.
    let text_to_embed = format!("{title}. {description}");
    info!("Generating embedding for article ID: {article_id} with text: \"{text_to_embed}\"");

    // 2. Generate the embedding.
    let vector = generate_embedding(embeddings_api_url, embeddings_model, &text_to_embed).await?;

    // 3. Convert Vec<f32> to &[u8] for BLOB storage.
    let vector_bytes: &[u8] =
        unsafe { std::slice::from_raw_parts(vector.as_ptr() as *const u8, vector.len() * 4) };

    // 4. Update the database record.
    conn.execute(
        "UPDATE articles SET embedding = ? WHERE id = ?",
        params![vector_bytes, article_id],
    )
    .await?;

    info!("Successfully embedded and updated article ID: {article_id}");
    Ok(())
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
) -> Result<Vec<SearchResult>, EmbeddingError> {
    let conn = db.connect().map_err(EmbeddingError::Database)?;

    let vector_str = format!(
        "vector('[{}]')",
        query_vector
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let sql = format!(
        "SELECT title, link, description, vector_distance_cos(embedding, {vector_str}) AS distance
         FROM articles
         WHERE embedding IS NOT NULL
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
) -> Result<Vec<SearchResult>, EmbeddingError> {
    let conn = db.connect().map_err(EmbeddingError::Database)?;
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
) -> Result<Vec<SearchResult>, EmbeddingError> {
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

    // Process vector results
    for (i, result) in vector_results.iter().enumerate() {
        let rank = (i + 1) as f64;
        *rrf_scores.entry(result.link.clone()).or_insert(0.0) += 1.0 / (k + rank);
    }

    // Process keyword results
    for (i, result) in keyword_results.iter().enumerate() {
        let rank = (i + 1) as f64;
        *rrf_scores.entry(result.link.clone()).or_insert(0.0) += 1.0 / (k + rank);
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
