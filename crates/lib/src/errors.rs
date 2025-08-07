use gcp_bigquery_client::error::BQError;
use thiserror::Error;

/// Custom error types for the application.
#[derive(Error, Debug)]
pub enum PromptError {
    #[error("Failed to build Reqwest client: {0}")]
    ReqwestClientBuild(reqwest::Error),
    #[error("Failed to send request to Gemini API: {0}")]
    GeminiRequest(reqwest::Error),
    #[error("Failed to deserialize Gemini API response: {0}")]
    GeminiDeserialization(reqwest::Error),
    #[error("Gemini API returned an error: {0}")]
    GeminiApi(String),
    #[error("Storage provider connection error: {0}")]
    StorageConnection(String),
    #[error("Storage query execution failed: {0}")]
    StorageQueryFailed(String),
    #[error("API key is missing")]
    MissingApiKey,
    #[error("Storage provider is missing")]
    MissingStorageProvider,
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
    #[error("Failed to serialize result to JSON: {0}")]
    JsonSerialization(#[from] serde_json::Error),
}

impl From<BQError> for PromptError {
    fn from(err: BQError) -> Self {
        PromptError::StorageQueryFailed(err.to_string())
    }
}
