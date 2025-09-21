use anyrag::{
    ingest::{EmbeddingError, KnowledgeError},
    search::SearchError,
    PromptError,
};
use anyrag_github::types::GitHubIngestError;
#[cfg(feature = "rss")]
use anyrag_rss::RssIngestError;
use anyrag_sheets::SheetError;
use anyrag_text::TextIngestError;
#[cfg(feature = "rss")]
use anyrag_web::WebIngestError;
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
    #[cfg(feature = "rss")]
    RssIngest(RssIngestError),

    /// Errors from the sheet ingestion process.
    Sheet(SheetError),
    /// Errors from the GitHub ingestion process.
    GitHubIngest(GitHubIngestError),
    /// Errors from the web ingestion process.
    WebIngest(WebIngestError),
    /// Errors from the embedding process.
    Embedding(EmbeddingError),
    /// Errors from the knowledge base pipeline.
    Knowledge(KnowledgeError),
    /// Errors from the search process.
    Search(SearchError),
    /// Errors from database operations.
    Database(TursoError),
    /// Errors from parsing JSON.
    JsonParse(serde_json::Error),
    /// Generic internal server errors.
    Internal(anyhow::Error),
}

/// Conversion from `serde_json::Error` to `AppError`.
impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::JsonParse(err)
    }
}

/// Conversion from `TextIngestError` to `AppError`.
impl From<TextIngestError> for AppError {
    fn from(err: TextIngestError) -> Self {
        AppError::TextIngest(err)
    }
}

/// Conversion from `RssIngestError` to `AppError`.
#[cfg(feature = "rss")]
impl From<RssIngestError> for AppError {
    fn from(err: RssIngestError) -> Self {
        AppError::RssIngest(err)
    }
}

/// Conversion from `SheetError` to `AppError`.
impl From<SheetError> for AppError {
    fn from(err: SheetError) -> Self {
        AppError::Sheet(err)
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

/// Conversion from `GitHubIngestError` to `AppError`.
impl From<GitHubIngestError> for AppError {
    fn from(err: GitHubIngestError) -> Self {
        AppError::GitHubIngest(err)
    }
}

/// Conversion from `WebIngestError` to `AppError`.
impl From<WebIngestError> for AppError {
    fn from(err: WebIngestError) -> Self {
        AppError::WebIngest(err)
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
            #[cfg(feature = "rss")]
            AppError::RssIngest(err) => {
                error!("RssIngestError: {:?}", err);
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    format!("Failed to ingest RSS feed: {err}"),
                )
            }

            AppError::Sheet(err) => {
                error!("SheetError: {:?}", err);
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    format!("Failed to process sheet: {err}"),
                )
            }
            AppError::WebIngest(err) => {
                error!("WebIngestError: {:?}", err);
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    format!("Failed to ingest from web: {err}"),
                )
            }
            AppError::Knowledge(err) => {
                error!("KnowledgeError: {:?}", err);
                let (status, msg) = match &err {
                    KnowledgeError::Parse(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to parse data: {e}"),
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
            AppError::GitHubIngest(err) => {
                error!("GitHubIngestError: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to ingest from GitHub: {err}"),
                )
            }
            AppError::Database(err) => {
                error!("Database error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Database operation failed: {err}"),
                )
            }
            AppError::JsonParse(err) => {
                error!("JsonParseError: {:?}", err);
                (
                    StatusCode::BAD_REQUEST,
                    format!("Failed to parse JSON response or payload: {err}"),
                )
            }
            AppError::Prompt(err) => {
                // Log the original error for debugging purposes
                error!("PromptError: {:?}", err);
                match err {
                    PromptError::MissingAiProvider(e) | PromptError::MissingStorageProvider(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Server configuration error: {e}"),
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
