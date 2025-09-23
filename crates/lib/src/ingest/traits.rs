use async_trait::async_trait;
use thiserror::Error;

/// A generic error type for all ingestion plugins.
///
/// Each plugin is responsible for mapping its specific errors (e.g., Git error, PDF parsing error)
/// into these standardized variants. This allows the core library to handle ingestion
/// errors in a uniform way.
#[derive(Error, Debug)]
pub enum IngestError {
    #[error("The specified source could not be found: {0}")]
    SourceNotFound(String),

    #[error("Failed to fetch or read content from the source: {0}")]
    Fetch(String),

    #[error("Failed to parse the content from the source: {0}")]
    Parse(String),

    #[error("A database operation failed during ingestion: {0}")]
    Database(#[from] turso::Error),

    #[error("An unexpected internal error occurred: {0}")]
    Internal(#[from] anyhow::Error),
}

/// Represents the successful result of an ingestion operation.
///
/// This struct provides a standardized summary of what was accomplished during an
/// ingestion task, which can be returned to the user or used for logging.
#[derive(Debug, Clone, Default)]
pub struct IngestionResult {
    /// The original source identifier (e.g., URL, file path) that was processed.
    pub source: String,
    /// The number of new documents or chunks successfully added to the database.
    pub documents_added: usize,
    /// A list of the unique IDs of the newly created documents.
    pub document_ids: Vec<String>,
    /// Optional field for returning extra context about the ingestion.
    /// This can be used for logging or for returning additional information to the user.
    /// It is a JSON string to allow for flexibility in the data that can be returned.
    pub metadata: Option<String>,
}

/// A generic trait that defines the contract for an ingestion plugin.
///
/// Any crate that provides a new data source for ingestion (e.g., GitHub, PDF, Web)
/// must implement this trait. This allows the core `anyrag-lib` to treat all
/// data sources polymorphically, making the system modular and extensible.
#[async_trait]
pub trait Ingestor: Send + Sync {
    /// The primary method for the trait.
    ///
    /// This function takes a source identifier (like a URL or file path) and an optional
    /// owner ID, performs the entire ingestion pipeline for that source, and returns
    //  a summary of the operation.
    ///
    /// # Arguments
    ///
    /// * `source`: The identifier for the content to ingest (e.g., "https://example.com").
    /// * `owner_id`: The ID of the user who owns the ingested content.
    async fn ingest(
        &self,
        source: &str,
        owner_id: Option<&str>,
    ) -> Result<IngestionResult, IngestError>;
}

/// A struct to hold the prompts for the knowledge ingestion pipeline.
/// This is passed to ingestors that use LLMs for content restructuring and metadata extraction.
#[derive(Debug, Clone, Copy)]
pub struct IngestionPrompts<'a> {
    pub restructuring_system_prompt: &'a str,
    pub metadata_extraction_system_prompt: &'a str,
}
