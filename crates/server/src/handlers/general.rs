//! # General Route Handlers
//!
//! This module contains the general-purpose Axum handlers for the `anyrag-server`,
//! including the root, health check, and the main Text-to-SQL prompt endpoint.

use super::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use anyrag::{
    ingest::{ingest_from_google_sheet_url, sheet_url_to_export_url_and_table_name},
    providers::db::storage::Storage,
    types::ContentType,
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

/// The primary handler for the `/prompt` endpoint.
///
/// This handler is now "intelligent", selecting the appropriate AI task from the
/// configuration based on the `content_type` provided in the request. If no
/// `content_type` is specified, it defaults to the `query_generation` task for
/// standard Text-to-SQL operations.
pub async fn prompt_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<Value>,
) -> Result<Json<ApiResponse<PromptResponse>>, AppError> {
    info!("Received prompt payload: '{}'", payload);
    let mut options: ExecutePromptOptions =
        serde_json::from_value(payload).map_err(anyrag::PromptError::from)?;

    // --- Task-based Configuration Loading ---
    // Select the task based on content_type, defaulting to query_generation.
    let task_name = match options.content_type {
        #[cfg(feature = "rss")]
        Some(ContentType::Rss) => "rss_summarization",
        Some(ContentType::Knowledge) => "rag_synthesis",
        _ => "query_generation",
    };
    info!("Selected task '{task_name}' based on content type.");

    let task_config = app_state.config.tasks.get(task_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "Configuration for task '{task_name}' not found."
        ))
    })?;

    // Get the specified AI provider for this task.
    let provider_name = &task_config.provider;
    let ai_provider = app_state.ai_providers.get(provider_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "Provider '{provider_name}' for task '{task_name}' not found in providers map."
        ))
    })?;

    // Apply the task's default prompts, allowing the request to override them.
    if options.system_prompt_template.is_none() {
        options.system_prompt_template = Some(task_config.system_prompt.clone());
    }
    if options.user_prompt_template.is_none() {
        options.user_prompt_template = Some(task_config.user_prompt.clone());
    }

    // Detect if a Google Sheet URL is present in the prompt.
    let sheet_url = options
        .prompt
        .split_whitespace()
        .find(|word| word.contains("/spreadsheets/d/"));

    let prompt_result = if let Some(url) = sheet_url {
        // --- Dynamic Google Sheet Querying Logic (Always uses SQLite) ---
        info!("Detected Google Sheet URL in prompt: {url}. Using SQLite provider.");
        let (export_url, table_name) = sheet_url_to_export_url_and_table_name(url)
            .map_err(|e| anyhow::anyhow!("Sheet URL transformation failed: {e}"))?;

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

        options.table_name = Some(table_name);
        let client = PromptClientBuilder::new()
            .ai_provider(ai_provider.clone())
            .storage_provider(Box::new(app_state.sqlite_provider.as_ref().clone()))
            .build()?;
        client.execute_prompt_with_options(options.clone()).await?
    } else if let Some(project_id) = options.project_id.as_deref() {
        // --- Dynamic BigQuery Client Creation ---
        info!("'project_id' provided. Creating a dynamic BigQuery client for this request.");
        #[cfg(feature = "bigquery")]
        {
            let bq_client = PromptClientBuilder::new()
                .ai_provider(ai_provider.clone())
                .bigquery_storage(project_id.to_string())
                .await?
                .build()?;
            bq_client
                .execute_prompt_with_options(options.clone())
                .await?
        }
        #[cfg(not(feature = "bigquery"))]
        {
            return Err(anyrag::PromptError::BigQueryFeatureNotEnabled.into());
        }
    } else {
        // --- Standard Querying Logic (Default SQLite) ---
        info!("No 'project_id' or sheet URL. Using default SQLite provider.");
        let client = PromptClientBuilder::new()
            .ai_provider(ai_provider.clone())
            .storage_provider(Box::new(app_state.sqlite_provider.as_ref().clone()))
            .build()?;
        client.execute_prompt_with_options(options.clone()).await?
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
