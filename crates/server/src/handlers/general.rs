//! # General Route Handlers
//!
//! This module contains the general-purpose Axum handlers for the `anyrag-server`,
//! including the root, health check, and the main Text-to-SQL prompt endpoint.

use super::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use anyrag::{
    ingest::{ingest_from_google_sheet_url, sheet_url_to_export_url_and_table_name},
    providers::db::storage::Storage,
    ExecutePromptOptions, PromptClientBuilder,
};
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::info;

// --- API Payloads for General Handlers ---

#[derive(Serialize, Deserialize)]
pub struct PromptResponse {
    pub text: String,
}

// --- General-Purpose Handlers ---

/// The handler for the root (`/`) endpoint.
pub async fn root() -> &'static str {
    "anyrag server is running."
}

/// The handler for the health check (`/health`) endpoint.
pub async fn health_check() -> &'static str {
    "OK"
}

/// The primary handler for the `/prompt` endpoint, which is the core of the
/// Text-to-SQL functionality.
pub async fn prompt_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<Value>,
) -> Result<Json<ApiResponse<PromptResponse>>, AppError> {
    info!("Received prompt payload: '{}'", payload);
    let mut options: ExecutePromptOptions =
        serde_json::from_value(payload).map_err(anyrag::PromptError::from)?;

    // Apply server-wide default prompts if not provided in the request.
    if options.system_prompt_template.is_none() {
        options.system_prompt_template = app_state.query_system_prompt_template.clone();
    }
    if options.user_prompt_template.is_none() {
        options.user_prompt_template = app_state.query_user_prompt_template.clone();
    }
    if options.format_system_prompt_template.is_none() {
        options.format_system_prompt_template = app_state.format_system_prompt_template.clone();
    }
    if options.format_user_prompt_template.is_none() {
        options.format_user_prompt_template = app_state.format_user_prompt_template.clone();
    }

    // Detect if a Google Sheet URL is present in the prompt.
    let sheet_url = options
        .prompt
        .split_whitespace()
        .find(|word| word.contains("/spreadsheets/d/"));

    let prompt_result = if let Some(url) = sheet_url {
        // --- Dynamic Google Sheet Querying Logic ---
        info!("Detected Google Sheet URL in prompt: {}", url);
        let (export_url, table_name) = sheet_url_to_export_url_and_table_name(url)
            .map_err(|e| anyhow::anyhow!("Sheet URL transformation failed: {e}"))?;

        // Ingest the sheet into the local SQLite DB if it's not already there.
        if app_state
            .sqlite_provider
            .get_table_schema(&table_name)
            .await
            .is_err()
        {
            info!("Table '{table_name}' does not exist. Starting ingestion.");
            ingest_from_google_sheet_url(&app_state.sqlite_provider.db, &export_url, &table_name)
                .await
                .map_err(|e| anyhow::anyhow!("Sheet ingestion failed: {e}"))?;
        } else {
            info!("Table '{table_name}' already exists. Skipping ingestion.");
        }

        // Temporarily override the prompt client to use the SQLite provider for this request.
        options.table_name = Some(table_name);
        let sqlite_prompt_client = PromptClientBuilder::new()
            .ai_provider(app_state.prompt_client.ai_provider.clone())
            .storage_provider(Box::new(app_state.sqlite_provider.as_ref().clone()))
            .build()?;
        sqlite_prompt_client
            .execute_prompt_with_options(options.clone())
            .await?
    } else {
        // --- Standard Querying Logic (e.g., BigQuery) ---
        app_state
            .prompt_client
            .execute_prompt_with_options(options.clone())
            .await?
    };

    let debug_info = if debug_params.debug.unwrap_or(false) {
        Some(json!({
            "options": options,
            "generated_sql": prompt_result.generated_sql,
            "database_result": prompt_result.database_result,
        }))
    } else {
        None
    };
    Ok(wrap_response(
        PromptResponse {
            text: prompt_result.text,
        },
        debug_params,
        debug_info,
    ))
}
