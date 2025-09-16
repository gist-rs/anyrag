//! # Google Sheets Ingestion Handler (Modern Pipeline)
//!
//! This module provides the unified handler for ingesting data from a Google Sheet.
//! It leverages the same modern, YAML-based knowledge pipeline as other ingestion sources.

use crate::{
    auth::middleware::AuthenticatedUser,
    handlers::{wrap_response, ApiResponse, AppError, AppState, DebugParams},
};
use anyrag::ingest::{
    knowledge::{extract_and_store_metadata, restructure_with_llm, IngestionPrompts},
    shared::{construct_export_url_and_table_name, download_csv},
};
use axum::{
    extract::{Query, State},
    Json,
};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct IngestSheetRequest {
    pub url: String,
    #[serde(default)]
    pub gid: Option<String>,
}

#[derive(Serialize)]
pub struct IngestSheetResponse {
    pub message: String,
    pub ingested_faqs: usize,
}

/// Unified handler for ingesting a Google Sheet into the knowledge base.
///
/// This handler orchestrates a pipeline that:
/// 1. Downloads the specified Google Sheet as CSV.
/// 2. Creates or updates a parent document for the sheet.
/// 3. Uses an LLM to restructure the CSV content into structured YAML.
/// 4. Updates the parent document's content with the YAML.
/// 5. Uses another LLM call to extract metadata from the YAML.
pub async fn ingest_sheet_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<IngestSheetRequest>,
) -> Result<Json<ApiResponse<IngestSheetResponse>>, AppError> {
    let owner_id = Some(user.0.id.clone());
    info!(
        "User '{:?}' initiating modern Sheet ingest for URL: {}",
        owner_id, payload.url
    );

    // --- 1. Download CSV content from Google Sheet ---
    let (export_url, _) = construct_export_url_and_table_name(&payload.url, payload.gid.as_deref())
        .map_err(AppError::from)?;
    let csv_content = download_csv(&export_url).await.map_err(AppError::from)?;
    // --- 2. Create or Update Parent Document ---
    let conn = app_state.sqlite_provider.db.connect()?;
    let document_id: String;

    if let Some(row) = conn
        .query(
            "SELECT id, content FROM documents WHERE source_url = ?",
            turso::params![payload.url.clone()],
        )
        .await?
        .next()
        .await?
    {
        // TODO: Re-implement content change detection. The previous hash comparison
        // was flawed because the stored content (YAML) was compared against new
        // raw content (CSV). For now, we re-process on every request.
        document_id = row.get(0)?;
    } else {
        document_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, payload.url.as_bytes()).to_string();
        let title = format!("Data from sheet: {}", payload.url);
        conn.execute(
            "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?, ?, ?, ?, ?)",
            turso::params![
                document_id.clone(),
                owner_id.clone(),
                payload.url.clone(),
                title,
                csv_content.clone() // Store raw CSV initially
            ],
        ).await?;
    }

    // --- 3. Restructure CSV to YAML using LLM ---
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

    let structured_yaml = restructure_with_llm(
        ai_provider.as_ref(),
        &csv_content,
        prompts.restructuring_system_prompt,
    )
    .await?;

    // --- 4. Update Document and Extract Metadata ---
    conn.execute(
        "UPDATE documents SET content = ? WHERE id = ?",
        turso::params![structured_yaml.clone(), document_id.clone()],
    )
    .await?;

    extract_and_store_metadata(
        &conn,
        ai_provider.as_ref(),
        &document_id,
        owner_id.as_deref(),
        &structured_yaml,
        prompts.metadata_extraction_system_prompt,
    )
    .await?;

    let response = IngestSheetResponse {
        message: "Sheet ingestion pipeline completed successfully.".to_string(),
        ingested_faqs: 1, // We now process the entire sheet as one knowledge document.
    };
    let debug_info = json!({
        "url": payload.url,
        "gid": payload.gid,
        "owner_id": owner_id,
        "document_id": document_id,
    });

    Ok(wrap_response(response, debug_params, Some(debug_info)))
}
