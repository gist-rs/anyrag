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
        // --- Dynamic Google Sheet Querying Logic (Always uses SQLite) ---
        info!(
            "Detected Google Sheet URL in prompt: {}. Using SQLite provider.",
            url
        );
        let (export_url, table_name) = sheet_url_to_export_url_and_table_name(url)
            .map_err(|e| anyhow::anyhow!("Sheet URL transformation failed: {e}"))?;

        // Ingest the sheet into the local SQLite DB if it's not already there.
        if app_state
            .sqlite_provider
            .get_table_schema(&table_name)
            .await
            .is_err()
        {
            info!("Table '{}' does not exist. Starting ingestion.", table_name);
            ingest_from_google_sheet_url(&app_state.sqlite_provider.db, &export_url, &table_name)
                .await
                .map_err(|e| anyhow::anyhow!("Sheet ingestion failed: {e}"))?;
        } else {
            info!("Table '{}' already exists. Skipping ingestion.", table_name);
        }

        // Temporarily override the table name and use the default SQLite client.
        options.table_name = Some(table_name);
        app_state
            .prompt_client
            .execute_prompt_with_options(options.clone())
            .await?
    } else if let Some(project_id) = options.project_id.as_deref() {
        // --- Dynamic BigQuery Client Creation ---
        info!("'project_id' provided. Creating a dynamic BigQuery client for this request.");

        #[cfg(feature = "bigquery")]
        {
            let bq_client = PromptClientBuilder::new()
                .ai_provider(app_state.prompt_client.ai_provider.clone())
                .bigquery_storage(project_id.to_string())
                .await?
                .build()?;
            bq_client
                .execute_prompt_with_options(options.clone())
                .await?
        }

        #[cfg(not(feature = "bigquery"))]
        {
            // If the feature is not enabled, we cannot fulfill the request.
            return Err(anyrag::PromptError::BigQueryFeatureNotEnabled.into());
        }
    } else {
        // --- Standard Querying Logic (Default SQLite) ---
        info!(
            "No 'project_id' or sheet URL provided. Using the default SQLite-based prompt client."
        );
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
