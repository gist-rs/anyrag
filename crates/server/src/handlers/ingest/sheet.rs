//! # Google Sheets Ingestion Handler
//!
//! This module provides the HTTP handler for ingesting data from a Google Sheet.
//! It acts as a thin web layer, orchestrating the call to the `anyrag-sheets`
//! crate through the generic `Ingestor` trait.

use crate::{
    auth::middleware::AuthenticatedUser,
    handlers::{wrap_response, ApiResponse, AppError, AppState, DebugParams},
};
use anyrag::ingest::{IngestionPrompts, Ingestor};
use anyrag_sheets::SheetsIngestor;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

#[derive(Deserialize)]
pub struct IngestSheetRequest {
    pub url: String,
    #[serde(default)]
    pub gid: Option<String>,
}

#[derive(Serialize)]
pub struct IngestSheetResponse {
    pub message: String,
    pub ingested_chunks: usize,
    pub document_ids: Vec<String>,
}

/// Handler for ingesting a Google Sheet using the `anyrag-sheets` plugin.
pub async fn ingest_sheet_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<IngestSheetRequest>,
) -> Result<Json<ApiResponse<IngestSheetResponse>>, AppError> {
    let owner_id = Some(user.0.id);
    info!(
        "User '{:?}' initiating Sheet ingest for URL: {}",
        owner_id, payload.url
    );

    // --- 1. Get dependencies from app state ---
    let task_name = "knowledge_distillation";
    let task_config = app_state.tasks.get(task_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("Task '{}' not found in config", task_name))
    })?;
    let provider_name = &task_config.provider;
    let ai_provider = app_state.ai_providers.get(provider_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("Provider '{}' not found", provider_name))
    })?;

    let meta_task_config = app_state
        .tasks
        .get("knowledge_metadata_extraction")
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Task 'knowledge_metadata_extraction' not found in config"
            ))
        })?;

    let prompts = IngestionPrompts {
        restructuring_system_prompt: &task_config.system_prompt,
        metadata_extraction_system_prompt: &meta_task_config.system_prompt,
    };

    // --- 2. Instantiate and call the ingestor plugin ---
    let ingestor =
        SheetsIngestor::new(&app_state.sqlite_provider.db, ai_provider.as_ref(), prompts);

    let source_json = json!({
        "url": payload.url,
        "gid": payload.gid,
    })
    .to_string();

    let ingest_result = ingestor
        .ingest(&source_json, owner_id.as_deref())
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Sheet ingestion failed: {e}")))?;

    // --- 3. Construct the response ---
    let debug_info = json!({
        "url": payload.url,
        "gid": payload.gid,
        "owner_id": owner_id,
        "document_id": ingest_result.document_ids.first(),
    });

    let response = IngestSheetResponse {
        message: "Sheet ingestion pipeline completed successfully.".to_string(),
        ingested_chunks: ingest_result.documents_added,
        document_ids: ingest_result.document_ids,
    };

    Ok(wrap_response(response, debug_params, Some(debug_info)))
}
