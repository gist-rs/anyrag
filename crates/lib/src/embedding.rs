//! # Vector Embedding and Search
//!
//! This module provides the core logic for generating vector embeddings for articles
//! and performing semantic similarity searches against them in the database.

use crate::providers::ai::embedding::generate_embedding;
use serde::Serialize;
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
#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub title: String,
    pub link: String,
    pub description: String,
    pub distance: f64,
}

/// Fetches an article, generates an embedding for it, and saves it to the database.
///
/// # Arguments
///
/// * `db`: A shared reference to the Turso database instance.
/// * `embeddings_api_url`: The URL of the embeddings API endpoint.
/// * `embeddings_model`: The name of the model to use for generating embeddings.
/// * `article_id`: The ID of the article to process.
pub async fn embed_and_update_article(
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

    let text_to_embed = format!("{title}. {description}");
    info!("Generating embedding for article ID: {article_id}");

    // 2. Generate the embedding.
    let vector = generate_embedding(embeddings_api_url, embeddings_model, &text_to_embed).await?;

    // 3. Convert Vec<f32> to &[u8] for BLOB storage.
    // This is a "zero-copy" conversion. It is unsafe but highly performant.
    // It relies on f32 being 4 bytes, which is standard.
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

/// Searches for articles similar to a given query vector.
///
/// # Arguments
///
/// * `db`: A shared reference to the Turso database instance.
/// * `query_vector`: The vector to compare against.
/// * `limit`: The maximum number of results to return.
pub async fn search_articles_by_embedding(
    db: &Database,
    query_vector: Vec<f32>,
    limit: u32,
) -> Result<Vec<SearchResult>, EmbeddingError> {
    let conn = db.connect().map_err(EmbeddingError::Database)?;

    // Turso's vector functions expect the vector as a string literal within the query.
    let vector_str = format!(
        "vector32('[{}]')",
        query_vector
            .into_iter()
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
        let distance: f64 = match row.get_value(3)? {
            TursoValue::Real(f) => f,
            _ => 0.0,
        };
        search_results.push(SearchResult {
            title,
            link,
            description,
            distance,
        });
    }

    Ok(search_results)
}
