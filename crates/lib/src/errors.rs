use gcp_bigquery_client::error::BQError;
use thiserror::Error;

/// Custom error types for the application.
#[derive(Error, Debug)]
pub enum PromptError {
    #[error("Failed to build Reqwest client: {0}")]
    ReqwestClientBuild(reqwest::Error),
    #[error("Failed to send request to the AI provider: {0}")]
    AiRequest(reqwest::Error),
    #[error("Failed to deserialize AI provider response: {0}")]
    AiDeserialization(reqwest::Error),
    #[error("AI provider returned an error: {0}")]
    AiApi(String),
    #[error("Storage provider connection error: {0}")]
    StorageConnection(String),
    #[error("Storage operation failed: {0}")]
    StorageOperationFailed(String),
    #[error("AI provider is missing")]
    MissingAiProvider,
    #[error("Storage provider is missing")]
    MissingStorageProvider,
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
    #[error("Failed to serialize result to JSON: {0}")]
    JsonSerialization(#[from] serde_json::Error),
}

impl From<BQError> for PromptError {
    fn from(err: BQError) -> Self {
        PromptError::StorageOperationFailed(err.to_string())
    }
}
