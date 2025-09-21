//! # Database Route Handlers
//!
//! This module contains handlers for direct database interaction endpoints.

use super::{wrap_response, ApiResponse, AppError, DebugParams};
use crate::state::AppState;
use anyrag::{
    constants,
    providers::db::{sqlite::SqliteProvider, storage::Storage},
};
use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::info;

// --- API Payloads for DB Handlers ---

#[derive(Deserialize, Debug)]
pub struct DbQueryRequest {
    pub db: String,
    pub query: String,
}

// --- DB Handlers ---

/// Handler for executing a raw, read-only SQL query against a specific project's database.
pub async fn db_query_handler(
    State(_app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<DbQueryRequest>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    info!("Received direct DB query for db: '{}'", payload.db);

    // Security: Validate that the query is read-only.
    let upper_query = payload.query.trim().to_uppercase();
    if !upper_query.starts_with("SELECT") && !upper_query.starts_with("PRAGMA") {
        return Err(AppError::Internal(anyhow::anyhow!(
            "Invalid query. Only read-only SELECT and PRAGMA queries are allowed."
        )));
    }

    // Dynamically create a provider for the requested project's database.
    let db_path = format!("{}/{}.db", constants::DB_DIR, payload.db);
    let sqlite_provider = SqliteProvider::new(&db_path).await?;
    sqlite_provider.initialize_schema().await?; // Ensure tables exist

    let result_json_str = sqlite_provider.execute_query(&payload.query).await?;
    let result_value: Value =
        serde_json::from_str(&result_json_str).map_err(anyhow::Error::from)?;

    let debug_info = json!({
        "db": payload.db,
        "query": payload.query,
    });

    Ok(wrap_response(result_value, debug_params, Some(debug_info)))
}
