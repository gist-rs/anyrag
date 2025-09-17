use crate::auth::middleware::AuthenticatedUser;
use crate::handlers::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use anyrag::ingest::Ingestor;
use anyrag_text::TextIngestor;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

#[derive(Deserialize)]
pub struct IngestTextRequest {
    pub text: String,
    #[serde(default = "default_source")]
    pub source: String,
}

fn default_source() -> String {
    "text_input".to_string()
}

#[derive(Serialize)]
pub struct IngestTextResponse {
    pub message: String,
    pub ingested_chunks: usize,
}

/// Handler for ingesting raw text content using the `anyrag-text` plugin.
pub async fn ingest_text_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<IngestTextRequest>,
) -> Result<Json<ApiResponse<IngestTextResponse>>, AppError> {
    let owner_id = Some(user.0.id);
    info!(
        "User '{:?}' sending text ingest request from source: {}",
        owner_id, payload.source
    );

    // 1. Instantiate the ingestor plugin.
    let ingestor = TextIngestor::new(&app_state.sqlite_provider.db);

    // 2. Serialize the source information into a JSON string for the generic ingest method.
    let source_json = json!({
        "text": payload.text,
        "source": payload.source
    })
    .to_string();

    // 3. Call the generic ingest method from the trait.
    let result = ingestor
        .ingest(&source_json, owner_id.as_deref())
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Text ingestion failed: {}", e)))?;

    // 4. Construct the final HTTP response.
    let message = if result.documents_added > 0 {
        format!(
            "Text ingestion successful. Stored {} new document chunks.",
            result.documents_added
        )
    } else {
        "No new text chunks were ingested. The content might be empty or already exist.".to_string()
    };

    let response = IngestTextResponse {
        message,
        ingested_chunks: result.documents_added,
    };
    let debug_info = json!({
        "source": result.source,
        "chunks_created": result.documents_added,
        "document_ids": result.document_ids,
        "owner_id": owner_id,
    });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}
