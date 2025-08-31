use anyrag::{
    ingest::{
        EmbeddingError, IngestSheetFaqError, KnowledgeError, RssIngestError, TextIngestError,
    },
    search::SearchError,
    PromptError,
};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tracing::error;
use turso::Error as TursoError;

/// A custom error type for the server application.
///
/// This enum encapsulates different kinds of errors that can occur within the server,
/// allowing them to be converted into appropriate HTTP responses.
#[derive(Debug)]
pub enum AppError {
    /// Errors originating from the `anyrag`.
    Prompt(PromptError),
    /// Errors from the text ingestion process.
    TextIngest(TextIngestError),
    /// Errors from the RSS ingestion process.
    RssIngest(RssIngestError),
    /// Errors from the sheet faq ingestion process.
    SheetFaqIngest(IngestSheetFaqError),

    /// Errors from the embedding process.
    Embedding(EmbeddingError),
    /// Errors from the knowledge base pipeline.
    Knowledge(KnowledgeError),
    /// Errors from the search process.
    Search(SearchError),
    /// Errors from database operations.
    Database(TursoError),
    /// Generic internal server errors.
    Internal(anyhow::Error),
}

/// Conversion from `TextIngestError` to `AppError`.
impl From<TextIngestError> for AppError {
    fn from(err: TextIngestError) -> Self {
        AppError::TextIngest(err)
    }
}

/// Conversion from `RssIngestError` to `AppError`.
impl From<RssIngestError> for AppError {
    fn from(err: RssIngestError) -> Self {
        AppError::RssIngest(err)
    }
}

/// Conversion from `IngestSheetFaqError` to `AppError`.
impl From<IngestSheetFaqError> for AppError {
    fn from(err: IngestSheetFaqError) -> Self {
        AppError::SheetFaqIngest(err)
    }
}

/// Conversion from `EmbeddingError` to `AppError`.
impl From<EmbeddingError> for AppError {
    fn from(err: EmbeddingError) -> Self {
        AppError::Embedding(err)
    }
}

/// Conversion from `KnowledgeError` to `AppError`.
impl From<KnowledgeError> for AppError {
    fn from(err: KnowledgeError) -> Self {
        AppError::Knowledge(err)
    }
}

/// Conversion from `SearchError` to `AppError`.
impl From<SearchError> for AppError {
    fn from(err: SearchError) -> Self {
        AppError::Search(err)
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

/// Conversion from `turso::Error` to `AppError`.
impl From<TursoError> for AppError {
    fn from(err: TursoError) -> Self {
        AppError::Database(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status_code, error_message) = match self {
            AppError::TextIngest(err) => {
                error!("TextIngestError: {:?}", err);
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    format!("Failed to ingest text: {err}"),
                )
            }
            AppError::RssIngest(err) => {
                error!("RssIngestError: {:?}", err);
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    format!("Failed to ingest RSS feed: {err}"),
                )
            }
            AppError::SheetFaqIngest(err) => {
                error!("IngestSheetFaqError: {:?}", err);
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    format!("Failed to ingest Sheet FAQ: {err}"),
                )
            }
            AppError::Knowledge(err) => {
                error!("KnowledgeError: {:?}", err);
                let (status, msg) = match &err {
                    KnowledgeError::JinaReaderFailed { status, body } => (
                        StatusCode::BAD_GATEWAY,
                        format!("Upstream fetch failed (status: {status}): {body}"),
                    ),
                    KnowledgeError::Llm(e) => {
                        (StatusCode::BAD_GATEWAY, format!("AI provider error: {e}"))
                    }
                    KnowledgeError::Parse(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to parse LLM response: {e}"),
                    ),
                    _ => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Knowledge base operation failed: {err}"),
                    ),
                };
                (status, msg)
            }
            AppError::Embedding(err) => {
                error!("EmbeddingError: {:?}", err);
                let status_code = match err {
                    EmbeddingError::NotFound(_) | EmbeddingError::FaqNotFound(_) => {
                        StatusCode::NOT_FOUND
                    }
                    EmbeddingError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
                    EmbeddingError::Embedding(_) => StatusCode::BAD_GATEWAY,
                };
                (status_code, format!("Embedding failed: {err}"))
            }
            AppError::Search(err) => {
                error!("SearchError: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Search operation failed: {err}"),
                )
            }
            AppError::Database(err) => {
                error!("Database error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Database operation failed: {err}"),
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
                    PromptError::BigQueryFeatureNotEnabled => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "The server is not configured for BigQuery. The 'bigquery' feature is not enabled.".to_string()
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
