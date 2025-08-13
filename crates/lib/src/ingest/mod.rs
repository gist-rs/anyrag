//! # Ingestion Logic
//!
//! This module provides the functionality for ingesting data from external sources,
//! such as RSS feeds, and storing it in a local database for later use in RAG.

pub mod embedding;
pub mod knowledge;
pub mod rss;
pub mod sheets;

pub use embedding::{embed_article, embed_faq, EmbeddingError};
pub use knowledge::{export_for_finetuning, run_ingestion_pipeline, KnowledgeError};
pub use rss::{ingest_from_url, IngestError};
pub use sheets::{
    ingest_from_google_sheet_url, sheet_url_to_export_url_and_table_name, IngestSheetError,
};
