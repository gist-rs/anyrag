//! # Ingestion Logic
//!
//! This module provides the functionality for ingesting data from external sources,
//! such as RSS feeds, text, and knowledge bases, and storing it in a local
//! database for later use in RAG.

pub mod embedding;
#[cfg(feature = "firebase")]
pub mod firebase;
pub mod knowledge;
pub mod pdf;
pub mod rss;
pub mod shared;
pub mod sheet_faq;
pub mod sheets;
pub mod text;

pub use embedding::{embed_article, EmbeddingError};
#[cfg(feature = "firebase")]
pub use firebase::{dump_firestore_collection, DumpFirestoreOptions, FirebaseIngestError};
pub use knowledge::{export_for_finetuning, run_ingestion_pipeline, KnowledgeError};
pub use pdf::{run_pdf_ingestion_pipeline, PdfSyncExtractor};
pub use rss::{ingest_from_url, IngestError as RssIngestError};
pub use sheet_faq::{ingest_faq_from_google_sheet, IngestSheetFaqError};
pub use sheets::{
    ingest_from_google_sheet_url, sheet_url_to_export_url_and_table_name, IngestSheetError,
};
pub use text::{chunk_text, IngestError as TextIngestError};
