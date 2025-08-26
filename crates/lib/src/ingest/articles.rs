//! # Shared Articles Ingestion Logic
//!
//! This module provides a centralized place for handling operations related to the
//! `articles` table in the database. It is used by different ingestion sources
//! (e.g., RSS, text) to ensure consistent data handling and to avoid code duplication.

use thiserror::Error;
use tracing::{info, warn};
use turso::{params, Connection, Value};

/// Custom error types for article ingestion.
#[derive(Error, Debug)]
pub enum ArticleError {
    #[error("Database error: {0}")]
    Database(#[from] turso::Error),
}

/// Represents a generic article to be inserted into the database.
/// This struct is source-agnostic.
pub struct Article {
    pub title: String,
    pub link: String,
    pub description: String,
    pub source_url: String,
    pub pub_date: Option<String>,
}

/// Creates the `articles` table in the database if it does not already exist.
/// This function is idempotent and can be called safely on every ingestion.
pub async fn create_articles_table_if_not_exists(conn: &Connection) -> Result<(), turso::Error> {
    let table_sql = "
        CREATE TABLE IF NOT EXISTS articles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            link TEXT NOT NULL UNIQUE,
            description TEXT,
            embedding BLOB,
            content TEXT,
            pub_date TEXT,
            source_url TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
    ";
    conn.execute(table_sql, ()).await?;

    Ok(())
}

/// Inserts a batch of articles into the database within a single transaction.
///
/// It handles duplicate articles gracefully by skipping them based on the `link`'s
/// UNIQUE constraint.
///
/// # Arguments
///
/// * `conn`: A reference to a Turso database connection.
/// * `articles`: A vector of `Article` structs to be inserted.
///
/// # Returns
///
/// A `Result` containing a vector of the `id`s of the articles that were newly inserted,
/// or an `ArticleError` on failure.
pub async fn insert_articles(
    conn: &Connection,
    articles: Vec<Article>,
) -> Result<Vec<i64>, ArticleError> {
    if articles.is_empty() {
        return Ok(Vec::new());
    }

    info!(
        "Starting database transaction to ingest {} articles.",
        articles.len()
    );
    conn.execute("BEGIN TRANSACTION", ()).await?;

    let mut new_article_ids = Vec::new();
    let mut stmt = conn
        .prepare(
            "INSERT INTO articles (title, link, description, pub_date, source_url) VALUES (?, ?, ?, ?, ?) RETURNING id",
        )
        .await?;

    for article in articles {
        let params = params![
            article.title,
            article.link.clone(),
            article.description,
            article.pub_date.unwrap_or_default(),
            article.source_url
        ];

        match stmt.query(params).await {
            Ok(mut result_set) => {
                if let Some(row) = result_set.next().await? {
                    if let Ok(Value::Integer(id)) = row.get_value(0) {
                        new_article_ids.push(id);
                    }
                }
            }
            Err(turso::Error::SqlExecutionFailure(msg))
                if msg.contains("UNIQUE constraint failed") =>
            {
                warn!("Skipping duplicate article: {}", article.link);
            }
            Err(e) => {
                // For any other database error, we should rollback and abort.
                conn.execute("ROLLBACK", ()).await?;
                return Err(ArticleError::Database(e));
            }
        }
    }

    conn.execute("COMMIT", ()).await?;
    info!(
        "Transaction committed. Ingested {} new articles.",
        new_article_ids.len()
    );

    Ok(new_article_ids)
}
