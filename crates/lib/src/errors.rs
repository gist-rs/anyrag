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
    #[error("BigQuery client error: {0}")]
    BigQueryClient(#[from] gcp_bigquery_client::error::BQError),
    #[error("BigQuery query execution failed: {0}")]
    BigQueryExecution(String),
    #[error("API key is missing")]
    MissingApiKey,
    #[error("BigQuery project ID is missing")]
    MissingProjectId,
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
}
