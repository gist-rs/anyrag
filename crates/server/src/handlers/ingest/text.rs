use crate::auth::middleware::AuthenticatedUser;
use crate::handlers::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use anyrag::ingest::text::{chunk_text, ingest_chunks_as_documents};
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

/// Handler for ingesting raw text content.
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
    let chunks = chunk_text(&payload.text)?;
    let total_chunks = chunks.len();

    let mut conn = app_state.sqlite_provider.db.connect()?;

    let new_document_ids =
        ingest_chunks_as_documents(&mut conn, chunks, &payload.source, owner_id.as_deref()).await?;
    let ingested_count = new_document_ids.len();

    let message = if ingested_count > 0 {
        format!("Text ingestion successful. Stored {ingested_count} new document chunks.",)
    } else if total_chunks > 0 {
        "All content may already exist. No new chunks were ingested.".to_string()
    } else {
        "No text chunks found to ingest.".to_string()
    };

    let response = IngestTextResponse {
        message,
        ingested_chunks: ingested_count,
    };
    let debug_info = json!({
        "source": payload.source,
        "chunks_created": ingested_count,
        "original_text_length": payload.text.len(),
        "document_ids": new_document_ids,
        "owner_id": owner_id,
    });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}
