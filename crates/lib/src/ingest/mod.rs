//! # Ingestion Logic
//!
//! This module provides the functionality for ingesting data from external sources,
//! such as RSS feeds, and storing it in a local database for later use in RAG.

use thiserror::Error;
use tracing::{info, warn};
use turso::{params, Connection, Database};

/// Custom error types for the ingestion process.
#[derive(Error, Debug)]
pub enum IngestError {
    #[error("Database error: {0}")]
    Database(#[from] turso::Error),
    #[error("Failed to fetch feed: {0}")]
    Fetch(#[from] reqwest::Error),
    #[error("Failed to parse RSS feed: {0}")]
    Parse(#[from] rss::Error),
    #[error("Database connection failed: {0}")]
    Connection(String),
}

/// Fetches an RSS feed from a URL and saves the articles to a SQLite database.
///
/// This function is the core of the ingestion process. It performs the following steps:
/// 1. Opens a connection to the specified SQLite database file using Turso.
/// 2. Ensures that an `articles` table with the correct schema exists.
/// 3. Fetches the content from the given `feed_url` asynchronously.
/// 4. Parses the content as an RSS feed.
/// 5. Iterates through the feed items and inserts new articles into the `articles` table
///    within a single transaction. It uses the article's link as a unique key to prevent duplicates.
///
/// # Arguments
///
/// * `db`: A shared reference to the Turso database instance.
/// * `feed_url`: The URL of the RSS feed to ingest.
///
/// # Returns
///
/// The number of *new* articles that were successfully inserted into the database.
pub async fn ingest_from_url(db: &Database, feed_url: &str) -> Result<usize, IngestError> {
    let conn = db
        .connect()
        .map_err(|e| IngestError::Connection(e.to_string()))?;

    create_table_if_not_exists(&conn).await?;

    info!("Fetching RSS feed from: {feed_url}");
    let content = reqwest::get(feed_url).await?.bytes().await?;
    let channel = rss::Channel::read_from(&content[..])?;

    info!("Starting database transaction to ingest articles.");
    conn.execute("BEGIN TRANSACTION", ()).await?;

    let mut insert_count = 0;
    let mut stmt = conn
        .prepare("INSERT INTO articles (title, link, description, pub_date, source_url) VALUES (?, ?, ?, ?, ?)")
        .await?;

    for item in channel.items() {
        if let (Some(title), Some(link)) = (item.title(), item.link()) {
            // Parse the pubDate (RFC 2822) and format it to a sortable ISO 8601 format.
            // This ensures that `ORDER BY pub_date` works chronologically, not alphabetically.
            let formatted_date = match item.pub_date() {
                Some(date_str) => chrono::DateTime::parse_from_rfc2822(date_str)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|_| date_str.to_string()), // Fallback to original on parse error
                None => String::new(),
            };

            let params = params!(
                title,
                link,
                item.description().unwrap_or_default(),
                formatted_date,
                channel.link()
            );

            match stmt.execute(params).await {
                Ok(changes) => {
                    if changes > 0 {
                        insert_count += 1;
                    }
                }
                Err(turso::Error::SqlExecutionFailure(msg))
                    if msg.contains("UNIQUE constraint failed") =>
                {
                    // This is an expected error for a duplicate link, so we ignore it and continue.
                    warn!("Skipping duplicate article: {}", link);
                }
                Err(e) => {
                    // For any other database error, we should rollback and abort.
                    conn.execute("ROLLBACK", ()).await?;
                    return Err(IngestError::Database(e));
                }
            }
        } else {
            warn!("Skipping RSS item due to missing title or link.");
        }
    }

    conn.execute("COMMIT", ()).await?;
    info!("Transaction committed. Ingested {insert_count} new articles.",);

    Ok(insert_count)
}

/// Creates the `articles` table in the database if it does not already exist.
async fn create_table_if_not_exists(conn: &Connection) -> Result<(), turso::Error> {
    let table_sql = "
        CREATE TABLE IF NOT EXISTS articles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            link TEXT NOT NULL,
            description TEXT,
            embedding BLOB,
            content TEXT,
            pub_date TEXT,
            source_url TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
    ";
    conn.execute(table_sql, ()).await?;

    let index_sql = "CREATE UNIQUE INDEX IF NOT EXISTS idx_articles_link ON articles(link);";
    conn.execute(index_sql, ()).await?;

    Ok(())
}
