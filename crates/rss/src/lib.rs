//! # `anyrag-rss`: RSS Ingestion Plugin
//!
//! This crate provides the logic for ingesting data from RSS feeds as a self-contained
//! plugin for the `anyrag` ecosystem. It implements the `Ingestor` trait from the
//! core `anyrag` library.

use anyhow::anyhow;
use anyrag::ingest::{IngestError, IngestionResult, Ingestor};
use async_trait::async_trait;
use rss::Channel;
use serde::Deserialize;
use thiserror::Error;
use tracing::info;
use turso::{params, Database};
use uuid::Uuid;

/// Custom error types for the RSS ingestion process.
#[derive(Error, Debug)]
pub enum RssIngestError {
    #[error("Database connection failed: {0}")]
    Database(#[from] turso::Error),
    #[error("Failed to fetch RSS feed: {0}")]
    Fetch(#[from] reqwest::Error),
    #[error("Failed to parse RSS feed: {0}")]
    Parse(#[from] rss::Error),
    #[error("Source deserialization failed: {0}")]
    SourceDeserialization(#[from] serde_json::Error),
}

/// A helper to convert the specific `RssIngestError` into the generic `anyrag::ingest::IngestError`.
impl From<RssIngestError> for IngestError {
    fn from(err: RssIngestError) -> Self {
        match err {
            RssIngestError::Database(e) => IngestError::Database(e),
            RssIngestError::Fetch(e) => IngestError::Fetch(e.to_string()),
            RssIngestError::Parse(e) => IngestError::Parse(e.to_string()),
            RssIngestError::SourceDeserialization(e) => {
                IngestError::Internal(anyhow!("Failed to deserialize source JSON: {}", e))
            }
        }
    }
}

/// Defines the structure of the JSON string passed to the `ingest` method.
#[derive(Deserialize)]
struct RssSource {
    url: String,
}

/// The `Ingestor` implementation for RSS feeds.
pub struct RssIngestor {
    db: Database,
}

impl RssIngestor {
    /// Creates a new `RssIngestor`.
    pub fn new(db: &Database) -> Self {
        Self { db: db.clone() }
    }
}

#[async_trait]
impl Ingestor for RssIngestor {
    /// Fetches an RSS feed, parses its items, and stores them as documents in the database.
    ///
    /// The `source` argument is expected to be a JSON string with a single `url` key,
    /// for example: `{"url": "https://example.com/feed.xml"}`.
    async fn ingest(
        &self,
        source: &str,
        owner_id: Option<&str>,
    ) -> Result<IngestionResult, IngestError> {
        let rss_source: RssSource = serde_json::from_str(source).map_err(RssIngestError::from)?;
        let feed_url = &rss_source.url;
        let mut conn = self.db.connect().map_err(RssIngestError::from)?;

        info!("Fetching RSS feed from: {}", feed_url);
        let content = reqwest::get(feed_url)
            .await
            .map_err(RssIngestError::from)?
            .error_for_status()
            .map_err(RssIngestError::from)?
            .bytes()
            .await
            .map_err(RssIngestError::from)?;
        let channel = Channel::read_from(&content[..]).map_err(RssIngestError::from)?;

        if channel.items().is_empty() {
            info!("RSS feed has no items to ingest.");
            return Ok(IngestionResult {
                source: feed_url.to_string(),
                ..Default::default()
            });
        }

        let tx = conn.transaction().await.map_err(RssIngestError::from)?;
        let mut new_document_ids = Vec::new();

        for item in channel.items() {
            if let (Some(title), Some(link)) = (item.title(), item.link()) {
                let document_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, link.as_bytes()).to_string();
                let description = item.description().unwrap_or_default();
                let content = format!("{title}\n\n{description}");

                // The `source_url` is the unique link of the RSS item itself.
                let mut stmt = tx
                    .prepare(
                        "INSERT INTO documents (id, owner_id, source_url, title, content)
                         VALUES (?, ?, ?, ?, ?)
                         ON CONFLICT(source_url) DO NOTHING",
                    )
                    .await
                    .map_err(RssIngestError::from)?;

                let changes = stmt
                    .execute(params![
                        document_id.clone(),
                        owner_id,
                        link.to_string(),
                        title.to_string(),
                        content
                    ])
                    .await
                    .map_err(RssIngestError::from)?;

                if changes > 0 {
                    new_document_ids.push(document_id);
                }
            }
        }

        tx.commit().await.map_err(RssIngestError::from)?;

        info!(
            "Transaction committed. Ingested {} new documents from RSS feed.",
            new_document_ids.len()
        );

        Ok(IngestionResult {
            documents_added: new_document_ids.len(),
            source: feed_url.to_string(),
            document_ids: new_document_ids,
        })
    }
}
