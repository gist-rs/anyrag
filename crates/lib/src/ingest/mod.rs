//! # Ingestion Logic
//!
//! This module provides the functionality for ingesting data from external sources,
//! such as RSS feeds, and storing it in a local database for later use in RAG.

pub mod embedding;
pub mod rss;

pub use embedding::{embed_article, EmbeddingError};
pub use rss::{ingest_from_url, IngestError};
