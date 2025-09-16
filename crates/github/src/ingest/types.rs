use anyrag::PromptError;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

/// Custom error types for the GitHub ingestion pipeline.
#[derive(Error, Debug)]
pub enum GitHubIngestError {
    #[error("Git operation failed: {0}")]
    Git(String),
    #[error("Database error: {0}")]
    Database(#[from] turso::Error),
    #[error("Prompt client error: {0}")]
    Prompt(#[from] PromptError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Failed to parse version from file: {0}")]
    VersionParsing(String),
    #[error("No suitable version found for repository: {0}")]
    VersionNotFound(String),
    #[error("An internal error occurred: {0}")]
    Internal(#[from] anyhow::Error),
}

/// Represents the source of an extracted code example, ordered by priority.
/// The `Ord` implementation allows us to easily find the highest-priority source.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExampleSourceType {
    /// Lowest priority: README.md files.
    Readme,
    /// Dedicated example files (e.g., in an `/examples` directory).
    ExampleFile,
    /// Doc comments within the source code.
    DocComment,
    /// Highest priority: Tests, as they are executable and verifiable.
    Test,
}

impl std::fmt::Display for ExampleSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExampleSourceType::Readme => write!(f, "readme"),
            ExampleSourceType::ExampleFile => write!(f, "example_file"),
            ExampleSourceType::DocComment => write!(f, "doc_comment"),
            ExampleSourceType::Test => write!(f, "test"),
        }
    }
}

impl FromStr for ExampleSourceType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "readme" => Ok(ExampleSourceType::Readme),
            "example_file" => Ok(ExampleSourceType::ExampleFile),
            "doc_comment" => Ok(ExampleSourceType::DocComment),
            "test" => Ok(ExampleSourceType::Test),
            _ => Err(()),
        }
    }
}

/// Represents a single, extracted code example from a repository.
/// This struct is what will be stored in the repository-specific database.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneratedExample {
    /// A unique, deterministic handle for the example (e.g., "test:tests/auth.rs:test_login_flow").
    pub example_handle: String,
    /// The actual code snippet.
    pub content: String,
    /// The file path from which the example was extracted, relative to the repo root.
    pub source_file: String,
    /// The type of source, used for prioritization during conflict resolution.
    pub source_type: ExampleSourceType,
    /// The version (Git tag, release, or hash) of the repository.
    pub version: String,
}

/// Represents a tracked repository in the main metadata database.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackedRepository {
    /// The unique name of the repository (e.g., "tursodatabase-turso").
    pub repo_name: String,
    /// The original Git URL.
    pub url: String,
    /// The file path to the repository-specific SQLite database.
    pub db_path: String,
}

/// Represents a task to ingest a specific version of a GitHub repository.
#[derive(Debug, Clone)]
pub struct IngestionTask {
    /// The URL of the repository to clone.
    pub url: String,
    /// An optional version (tag, branch, commit hash) to check out.
    /// If `None`, the latest version will be determined and used.
    pub version: Option<String>,
    /// The API URL for the embedding model.
    pub embedding_api_url: Option<String>,
    /// The name of the embedding model to use.
    pub embedding_model: Option<String>,
    /// The API key for the embedding model.
    pub embedding_api_key: Option<String>,
}
