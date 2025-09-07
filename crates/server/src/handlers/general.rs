//! # General Route Handlers
//!
//! This module contains the general-purpose Axum handlers for the `anyrag-server`,
//! including the root, health check, and the main Text-to-SQL prompt endpoint.

use super::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use anyrag::{
    ingest::{ingest_from_google_sheet_url, sheet_url_to_export_url_and_table_name},
    providers::{ai::AiProvider, db::storage::Storage},
    types::{ContentType, ExecutePromptOptions, PromptResult},
    PromptClientBuilder,
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

/// A helper function to handle the specific logic for on-the-fly sheet ingestion within a prompt.
async fn handle_sheet_url_in_prompt(
    app_state: &AppState,
    url: &str,
    ai_provider: Box<dyn AiProvider>,
    mut options: ExecutePromptOptions,
) -> Result<PromptResult, AppError> {
    info!(
        "Detected Google Sheet URL in prompt: {}. Using SQLite provider.",
        url
    );
    let (export_url, table_name) = sheet_url_to_export_url_and_table_name(url)
        .map_err(|e| anyhow::anyhow!("Sheet URL transformation failed: {e}"))?;

    // Check if the table already exists to avoid re-ingesting.
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

    // Update options with the ingested table name and execute the prompt.
    options.table_name = Some(table_name);
    let client = PromptClientBuilder::new()
        .ai_provider(ai_provider)
        .storage_provider(Box::new(app_state.sqlite_provider.as_ref().clone()))
        .build()?;

    Ok(client.execute_prompt_with_options(options).await?)
}

/// The primary handler for the `/prompt` endpoint.
pub async fn prompt_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<Value>,
) -> Result<Json<ApiResponse<PromptResponse>>, AppError> {
    info!("Received prompt payload: '{}'", payload);
    let mut options: ExecutePromptOptions =
        serde_json::from_value(payload).map_err(anyrag::PromptError::from)?;

    // --- Shorthand "ls" command: Always targets a local DB ---
    if options.prompt.starts_with("ls ") {
        info!("Shorthand 'ls' command detected. Overriding to local DB query.");
        let parts: Vec<&str> = options.prompt.split_whitespace().collect();
        let table_name = match parts.get(1) {
            Some(tn) => tn.to_string(),
            None => {
                return Err(AppError::Internal(anyhow::anyhow!(
                    "'ls' command requires a table name."
                )));
            }
        };

        let mut limit = 10; // Default limit
        if let Some(limit_part) = parts.get(2) {
            if let Some(limit_str) = limit_part.strip_prefix("limit=") {
                limit = limit_str.parse().unwrap_or(10);
            }
        }

        // Determine DB name: prefer `db` field, fallback to `project_id` for this specific command.
        let db_name = options
            .db
            .as_deref()
            .or(options.project_id.as_deref())
            .ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!(
                    "'ls' command requires a 'db' or 'project_id' field."
                ))
            })?;

        // Update options for the prompt execution.
        options.table_name = Some(table_name.clone());
        options.prompt = format!(
            "List the first {limit} rows from the `{table_name}` table, showing all columns."
        );
        info!("Transformed prompt: '{}'", options.prompt);

        // --- Execute the transformed prompt directly, bypassing main logic ---

        // Get the AI provider for query generation.
        let task_config = app_state
            .config
            .tasks
            .get("query_generation")
            .ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!(
                    "Configuration for task 'query_generation' not found."
                ))
            })?;
        let provider_name = &task_config.provider;
        let ai_provider = app_state.ai_providers.get(provider_name).ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Provider '{provider_name}' for task 'query_generation' not found."
            ))
        })?;

        // Create a dynamic SQLite client for the specified project DB.
        let db_path = format!("db/{db_name}.db");
        let provider = anyrag::providers::db::sqlite::SqliteProvider::new(&db_path).await?;
        let client = PromptClientBuilder::new()
            .ai_provider(ai_provider.clone())
            .storage_provider(Box::new(provider))
            .build()?;

        let prompt_result = client.execute_prompt_with_options(options.clone()).await?;

        let debug_info = if debug_params.debug.unwrap_or(false) {
            Some(json!({
                "options": options,
                "generated_sql": prompt_result.generated_sql,
                "database_result": prompt_result.database_result,
            }))
        } else {
            None
        };

        return Ok(wrap_response(
            PromptResponse {
                text: prompt_result.text,
            },
            debug_params,
            debug_info,
        ));
    }

    // --- Task-based Configuration Loading ---
    // This logic determines which set of prompts and which AI provider to use.
    let task_name = match options.content_type {
        // 1. If a specific content_type is provided, use its dedicated task.
        #[cfg(feature = "rss")]
        Some(ContentType::Rss) => "rss_summarization",
        Some(ContentType::Knowledge) => "rag_synthesis",
        // 2. If no content_type, decide based on whether it's a query or direct generation.
        _ => {
            if options.table_name.is_some() || options.project_id.is_some() || options.db.is_some()
            {
                // If a table, project, or db is specified, it's a query task.
                "query_generation"
            } else {
                // Otherwise, it's a general-purpose text generation task.
                "direct_generation"
            }
        }
    };
    info!("Selected task '{task_name}' based on request payload.");

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
        handle_sheet_url_in_prompt(&app_state, url, ai_provider.clone(), options.clone()).await?
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
    } else if let Some(db_name) = options.db.as_deref() {
        // --- Dynamic SQLite Client Creation for a specific project DB ---
        info!(
            "'db' provided: '{}'. Creating a dynamic SQLite client for this request.",
            db_name
        );
        let db_path = format!("db/{db_name}.db");
        let provider = anyrag::providers::db::sqlite::SqliteProvider::new(&db_path).await?;
        let client = PromptClientBuilder::new()
            .ai_provider(ai_provider.clone())
            .storage_provider(Box::new(provider))
            .build()?;
        client.execute_prompt_with_options(options.clone()).await?
    } else {
        // --- Standard Querying Logic (Default SQLite for non-db tasks) ---
        info!("No 'project_id', 'db', or sheet URL. Using default SQLite provider.");
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
