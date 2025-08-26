//! # RSS Ingestion Logic
//!
//! This module provides the functionality for ingesting data from RSS feeds
//! and storing it in a local database.

use super::{create_articles_table_if_not_exists, insert_articles, Article, ArticleError};
use thiserror::Error;
use tracing::info;
use turso::Database;

/// Custom error types for the ingestion process.
#[derive(Error, Debug)]
pub enum IngestError {
    #[error("Database connection failed: {0}")]
    Connection(String),
    #[error("Failed to fetch feed: {0}")]
    Fetch(#[from] reqwest::Error),
    #[error("Failed to parse RSS feed: {0}")]
    Parse(#[from] rss::Error),
    #[error("Article insertion failed: {0}")]
    Article(#[from] ArticleError),
    #[error("Database setup failed: {0}")]
    DbSetup(#[from] turso::Error),
}

/// Fetches an RSS feed from a URL and saves the articles to a SQLite database.
///
/// This function is the core of the ingestion process. It performs the following steps:
/// 1. Ensures that an `articles` table with the correct schema exists.
/// 2. Fetches and parses the content from the given `feed_url`.
/// 3. Transforms the feed items into a generic `Article` format.
/// 4. Calls the shared `insert_articles` function to store them in the database.
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
    let mut conn = db
        .connect()
        .map_err(|e| IngestError::Connection(e.to_string()))?;

    // Use the shared function to ensure the table and index exist.
    create_articles_table_if_not_exists(&conn).await?;

    info!("Fetching RSS feed from: {feed_url}");
    let content = reqwest::get(feed_url).await?.bytes().await?;
    let channel = rss::Channel::read_from(&content[..])?;

    // Transform rss::Item into the shared ingest::Article struct.
    let articles_to_insert: Vec<Article> = channel
        .items()
        .iter()
        .filter_map(|item| {
            if let (Some(title), Some(link)) = (item.title(), item.link()) {
                // Parse the pubDate (RFC 2822) and format it to a sortable ISO 8601 format.
                let formatted_date = item.pub_date().and_then(|date_str| {
                    chrono::DateTime::parse_from_rfc2822(date_str)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .ok() // Convert Result to Option, discarding the error
                });

                Some(Article {
                    title: title.to_string(),
                    link: link.to_string(),
                    description: item.description().unwrap_or_default().to_string(),
                    source_url: channel.link().to_string(),
                    pub_date: formatted_date,
                })
            } else {
                None
            }
        })
        .collect();

    // Use the shared function to insert the articles.
    let new_ids = insert_articles(&mut conn, articles_to_insert).await?;

    Ok(new_ids.len())
}
