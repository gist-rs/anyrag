//! # Ingestion Logic
//!
//! This module provides the functionality for ingesting data from external sources,
//! such as RSS feeds, text, and knowledge bases, and storing it in a local
//! database for later use in RAG.

pub mod embedding;

pub mod knowledge;

#[cfg(feature = "sheets")]
pub mod shared;

pub mod state_manager;

pub mod traits;

pub mod types;

pub use embedding::{embed_article, EmbeddingError};

pub use knowledge::{export_for_finetuning, KnowledgeError};

pub use traits::{IngestError, IngestionPrompts, IngestionResult, Ingestor};
pub use types::{ContentMetadata, MetadataResponse};
