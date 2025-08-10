use anyrag::ingest::IngestError;
use anyrag::PromptError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tracing::error;

/// A custom error type for the server application.
///
/// This enum encapsulates different kinds of errors that can occur within the server,
/// allowing them to be converted into appropriate HTTP responses.
pub enum AppError {
    /// Errors originating from the `anyrag`.
    Prompt(PromptError),
    /// Errors from the ingestion process.
    Ingest(IngestError),
    /// Generic internal server errors.
    Internal(anyhow::Error),
}

/// Conversion from `IngestError` to `AppError`.
impl From<IngestError> for AppError {
    fn from(err: IngestError) -> Self {
        AppError::Ingest(err)
    }
}

/// Conversion from `PromptError` to `AppError`.
impl From<PromptError> for AppError {
    fn from(err: PromptError) -> Self {
        AppError::Prompt(err)
    }
}

/// Conversion from `anyhow::Error` to `AppError`.
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status_code, error_message) = match self {
            AppError::Ingest(err) => {
                error!("IngestError: {:?}", err);
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    format!("Failed to ingest data: {err}"),
                )
            }
            AppError::Prompt(err) => {
                // Log the original error for debugging purposes
                error!("PromptError: {:?}", err);
                match err {
                    PromptError::MissingAiProvider | PromptError::MissingStorageProvider => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Server is not configured correctly.".to_string(),
                    ),
                    PromptError::AiRequest(e) => (
                        StatusCode::BAD_GATEWAY,
                        format!("Request to AI provider failed: {e}"),
                    ),
                    PromptError::AiDeserialization(e) => (
                        StatusCode::BAD_GATEWAY,
                        format!("Failed to deserialize AI provider response: {e}"),
                    ),
                    PromptError::AiApi(e) => {
                        (StatusCode::BAD_GATEWAY, format!("AI provider error: {e}"))
                    }
                    PromptError::StorageConnection(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Storage provider connection error: {e}"),
                    ),
                    PromptError::StorageOperationFailed(e) => (
                        StatusCode::BAD_REQUEST,
                        format!("Storage operation failed: {e}"),
                    ),
                    PromptError::Regex(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Internal regex error: {e}"),
                    ),
                    PromptError::JsonSerialization(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to serialize result: {e}"),
                    ),
                    PromptError::ReqwestClientBuild(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to build HTTP client: {e}"),
                    ),
                }
            }
            AppError::Internal(err) => {
                error!("Internal server error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "An internal server error occurred.".to_string(),
                )
            }
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status_code, body).into_response()
    }
}
