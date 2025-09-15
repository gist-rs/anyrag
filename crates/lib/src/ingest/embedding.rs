//! # Embedding Generation for Ingested Data
//!
//! This module provides the logic for generating vector embeddings for data
//! that has been ingested, such as articles from an RSS feed. This is a key
//! step in preparing the data for semantic search.

use crate::providers::ai::generate_embedding;
use thiserror::Error;
use tracing::info;
use turso::{params, Database, Value as TursoValue};

/// Custom error types for the embedding process.
#[derive(Error, Debug)]
pub enum EmbeddingError {
    #[error("Database error: {0}")]
    Database(#[from] turso::Error),
    #[error("Embedding generation failed: {0}")]
    Embedding(#[from] crate::errors::PromptError),
    #[error("Article with ID {0} not found.")]
    NotFound(i64),
    #[error("FAQ with ID {0} not found.")]
    FaqNotFound(i64),
}

/// Fetches an article, generates an embedding for it, and saves it to the database.
///
/// This function is designed to process a single article at a time, making it suitable
/// for concurrent execution.
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
    embeddings_api_key: Option<&str>,
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
    let vector = generate_embedding(
        embeddings_api_url,
        embeddings_model,
        &text_to_embed,
        embeddings_api_key,
    )
    .await?;

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
