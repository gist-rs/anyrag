//! # Shared Articles Ingestion Logic
//!
//! This module provides a centralized place for handling operations related to the
//! `articles` table in the database. It is used by different ingestion sources
//! (e.g., RSS, text) to ensure consistent data handling and to avoid code duplication.

use thiserror::Error;
use tracing::info;
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
/// The SQL statement to create the `articles` table.
pub const CREATE_ARTICLES_TABLE_SQL: &str = "
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

/// Creates the `articles` table in the database if it does not already exist.
/// This function is idempotent and can be called safely on every ingestion.
pub async fn create_articles_table_if_not_exists(conn: &Connection) -> Result<(), turso::Error> {
    conn.execute(CREATE_ARTICLES_TABLE_SQL, ()).await?;
    Ok(())
}

/// Inserts a batch of articles into the database within a single transaction.
///
/// This function uses an `INSERT ... ON CONFLICT DO NOTHING` statement, which
/// allows it to efficiently and atomically skip any articles that already exist
/// in the database based on the `link`'s UNIQUE constraint.
///
/// # Arguments
///
/// * `conn`: A mutable reference to a Turso database connection.
/// * `articles`: A vector of `Article` structs to be inserted.
///
/// # Returns
///
/// A `Result` containing a vector of the `id`s of the articles that were newly inserted,
/// or an `ArticleError` on failure.
pub async fn insert_articles(
    conn: &mut Connection,
    articles: Vec<Article>,
) -> Result<Vec<i64>, ArticleError> {
    if articles.is_empty() {
        return Ok(Vec::new());
    }

    info!(
        "Starting database transaction to ingest {} articles.",
        articles.len()
    );
    let tx = conn.transaction().await?;
    let mut new_article_ids = Vec::new();

    for article in articles {
        // Prepare the statement inside the loop. This is more robust than reusing
        // the statement, as it avoids potential driver issues with parameter binding in a loop.
        let mut stmt = tx
            .prepare(
                "INSERT INTO articles (title, link, description, pub_date, source_url)
                 VALUES (?, ?, ?, ?, ?)
                 ON CONFLICT(link) DO NOTHING
                 RETURNING id",
            )
            .await?;

        let params = params![
            article.title,
            article.link,
            article.description,
            article.pub_date.unwrap_or_default(),
            article.source_url
        ];

        let mut result_set = stmt.query(params).await?;

        if let Some(row) = result_set.next().await? {
            if let Ok(Value::Integer(id)) = row.get_value(0) {
                new_article_ids.push(id);
            }
        }
    }

    tx.commit().await?;

    info!(
        "Transaction committed. Ingested {} new articles.",
        new_article_ids.len()
    );

    Ok(new_article_ids)
}
