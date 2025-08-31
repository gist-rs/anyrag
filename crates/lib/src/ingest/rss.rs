//! # RSS Ingestion Logic
//!
//! This module provides the functionality for ingesting data from RSS feeds
//! and storing it in the new, normalized `documents` table.

use thiserror::Error;
use tracing::info;
use turso::{params, Database};
use uuid::Uuid;

/// Custom error types for the RSS ingestion process.
#[derive(Error, Debug)]
pub enum IngestError {
    #[error("Database connection failed: {0}")]
    Database(#[from] turso::Error),
    #[error("Failed to fetch RSS feed: {0}")]
    Fetch(#[from] reqwest::Error),
    #[error("Failed to parse RSS feed: {0}")]
    Parse(#[from] rss::Error),
}

/// Fetches an RSS feed from a URL and saves the articles to the `documents` table.
///
/// This function is the core of the RSS ingestion process. It performs the following steps:
/// 1. Fetches and parses the content from the given `feed_url`.
/// 2. Transforms the feed items into a format suitable for the `documents` table.
/// 3. Inserts the new documents within a single transaction, skipping any that already
///    exist based on the `source_url` (the item's link).
///
/// # Arguments
///
/// * `db`: A shared reference to the Turso database instance.
/// * `feed_url`: The URL of the RSS feed to ingest.
///
/// # Returns
///
/// The number of *new* documents that were successfully inserted into the database.
pub async fn ingest_from_url(db: &Database, feed_url: &str) -> Result<usize, IngestError> {
    let mut conn = db.connect()?;

    info!("Fetching RSS feed from: {feed_url}");
    let content = reqwest::get(feed_url).await?.bytes().await?;
    let channel = rss::Channel::read_from(&content[..])?;

    if channel.items().is_empty() {
        info!("RSS feed has no items to ingest.");
        return Ok(0);
    }

    let tx = conn.transaction().await?;
    let mut insert_count = 0;

    for item in channel.items() {
        if let (Some(title), Some(link)) = (item.title(), item.link()) {
            let document_id = Uuid::new_v4().to_string();
            let description = item.description().unwrap_or_default();
            let content = format!("{title}\n\n{description}");

            // The `source_url` is the unique link of the RSS item itself.
            let mut stmt = tx
                .prepare(
                    "INSERT INTO documents (id, source_url, title, content)
                     VALUES (?, ?, ?, ?)
                     ON CONFLICT(source_url) DO NOTHING",
                )
                .await?;

            let changes = stmt
                .execute(params![
                    document_id,
                    link.to_string(),
                    title.to_string(),
                    content
                ])
                .await?;

            insert_count += changes as usize;
        }
    }

    tx.commit().await?;

    info!(
        "Transaction committed. Ingested {} new documents from RSS feed.",
        insert_count
    );

    Ok(insert_count)
}
