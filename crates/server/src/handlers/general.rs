//! # General Route Handlers
//!
//! This module contains the general-purpose Axum handlers for the `anyrag-server`,
//! including the root, health check, and the main Text-to-SQL prompt endpoint.

use super::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use anyrag::HttpRequestPromptOptions;
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
    pub text: Value,
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
pub async fn prompt_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<Value>,
) -> Result<Json<ApiResponse<PromptResponse>>, AppError> {
    info!("Received prompt payload: '{}'", payload);
    let server_options: HttpRequestPromptOptions =
        serde_json::from_value(payload).map_err(anyrag::PromptError::from)?;

    // All business logic is now delegated to the library crate.
    // The library will handle shorthand commands, dynamic provider creation, etc.
    // All business logic is now delegated to the library's executor.
    let prompt_result = app_state
        .executor
        .execute_http_prompt(server_options.clone())
        .await?;

    let debug_info = if debug_params.debug.unwrap_or(false) {
        Some(json!({
            "options": server_options,
            // "model_used" is now determined within the lib crate.
            "generated_sql": prompt_result.generated_sql,
            "database_result": prompt_result.database_result,
        }))
    } else {
        None
    };

    Ok(wrap_response(
        PromptResponse {
            text: Value::String(prompt_result.text),
        },
        debug_params,
        debug_info,
    ))
}
