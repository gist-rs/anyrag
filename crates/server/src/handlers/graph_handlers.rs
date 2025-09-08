#![cfg(feature = "graph_db")]

//! # Knowledge Graph Route Handlers
//!
//! This module contains handlers for endpoints that interact with the in-memory
//! Knowledge Graph, such as building it from a local database.

use super::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use crate::auth::middleware::AuthenticatedUser;
use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use std::path::Path;
use tracing::{debug, info};
use turso::Value as TursoValue;

// --- API Payloads for Graph Handlers ---

#[derive(Deserialize, Debug)]
pub struct GraphBuildRequest {
    pub db: String,
    pub table_name: String,
}

#[derive(serde::Serialize, Debug)]
pub struct GraphBuildResponse {
    pub message: String,
    pub facts_added: usize,
}

// --- Graph Handlers ---

// This struct is updated to handle potential nulls from the AI response.
#[derive(Deserialize, Debug, serde::Serialize)]
struct AiFactMapping {
    subject_column: Option<String>,
    predicate_name: Option<String>,
    object_column: Option<String>,
}

/// Handler for building or updating the in-memory Knowledge Graph from a local database table.
/// This handler intelligently maps columns from the source table to graph facts using an AI.
pub async fn graph_build_handler(
    State(app_state): State<AppState>,
    _user: AuthenticatedUser, // Ensures the endpoint is protected
    debug_params: Query<DebugParams>,
    Json(payload): Json<GraphBuildRequest>,
) -> Result<Json<ApiResponse<GraphBuildResponse>>, AppError> {
    info!(
        "Received request to build graph from db '{}', table '{}'",
        payload.db, payload.table_name
    );

    // 1. Connect to the specified database
    let db_path = format!("db/{}.db", payload.db);
    if !Path::new(&db_path).exists() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "Database file for project '{}' not found at '{}'",
            payload.db,
            db_path
        )));
    }
    let provider = anyrag::providers::db::sqlite::SqliteProvider::new(&db_path).await?;
    let conn = provider.db.connect()?;

    // 2. Inspect the table schema
    let pragma_sql = format!("PRAGMA table_info({})", payload.table_name);
    let mut pragma_rows = conn.query(&pragma_sql, ()).await?;
    let mut schema_parts = Vec::new();
    while let Some(row) = pragma_rows.next().await? {
        let name: String = row.get(1)?;
        let dtype: String = row.get(2)?;
        schema_parts.push(format!("{name} ({dtype})"));
    }
    let schema_string = schema_parts.join(", ");

    // 3. Use an LLM to determine the column mapping
    let task_config = app_state
        .tasks
        .get("direct_generation")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Task 'direct_generation' not found")))?;
    let ai_provider = app_state
        .ai_providers
        .get(&task_config.provider)
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Provider '{}' not found",
                task_config.provider
            ))
        })?;

    let system_prompt = "You are a data architect. Given a table schema, determine the best mapping for a knowledge graph fact (subject, predicate, object). Respond with a single JSON object containing `subject_column`, `predicate_name`, and `object_column` keys. The predicate should be a concise, descriptive name for the relationship (e.g., 'has_rating', 'is_about'). If a clear mapping is not possible, return null for the values.";
    let user_prompt = format!("# Table Schema\n{schema_string}");
    info!(
        "Sending user prompt to AI for graph mapping:\n---\n{}\n---",
        user_prompt
    );
    let ai_response = ai_provider.generate(system_prompt, &user_prompt).await?;
    let cleaned_response = ai_response
        .trim()
        .strip_prefix("```json")
        .unwrap_or(&ai_response)
        .strip_suffix("```")
        .unwrap_or(&ai_response)
        .trim();
    let mapping: AiFactMapping = serde_json::from_str(cleaned_response).map_err(|e| {
        AppError::Internal(anyhow::anyhow!(
            "Failed to parse AI mapping JSON: {e}. Raw response: {cleaned_response}"
        ))
    })?;

    // 4. Validate the mapping from the AI
    let (subject_column, predicate_name, object_column) = match (
        mapping.subject_column,
        mapping.predicate_name,
        mapping.object_column,
    ) {
        (Some(s), Some(p), Some(o)) if !s.is_empty() && !p.is_empty() && !o.is_empty() => (s, p, o),
        _ => {
            return Err(AppError::Internal(anyhow::anyhow!(
                    "AI could not determine a valid subject-predicate-object mapping from the table schema. Raw AI response: {}",
                    cleaned_response
                )));
        }
    };
    debug!(
        "AI suggested mapping: subject='{}', predicate='{}', object='{}'",
        subject_column, predicate_name, object_column
    );

    // 5. Build and execute a dynamic query based on the mapping
    let dynamic_sql = format!(
        "SELECT \"{}\", \"{}\" FROM {}",
        subject_column, object_column, payload.table_name
    );
    let mut data_rows = conn.query(&dynamic_sql, ()).await?;

    // 6. Create "timeless" facts from the query results
    let mut facts_to_add = Vec::new();
    let start_time = DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z")
        .map_err(anyhow::Error::from)?
        .with_timezone(&Utc);
    let end_time = DateTime::parse_from_rfc3339("9999-12-31T23:59:59Z")
        .map_err(anyhow::Error::from)?
        .with_timezone(&Utc);

    while let Some(row) = data_rows.next().await? {
        let subject_val: TursoValue = row.get_value(0)?;
        let object_val: TursoValue = row.get_value(1)?;

        // Explicitly convert TursoValue to String to handle different types.
        let subject_str = match subject_val {
            TursoValue::Text(s) => s,
            TursoValue::Integer(i) => i.to_string(),
            TursoValue::Real(f) => f.to_string(),
            TursoValue::Blob(_) => "[BLOB]".to_string(),
            TursoValue::Null => "NULL".to_string(),
        };
        let object_str = match object_val {
            TursoValue::Text(s) => s,
            TursoValue::Integer(i) => i.to_string(),
            TursoValue::Real(f) => f.to_string(),
            TursoValue::Blob(_) => "[BLOB]".to_string(),
            TursoValue::Null => "NULL".to_string(),
        };

        facts_to_add.push((
            subject_str,
            predicate_name.clone(),
            object_str,
            start_time,
            end_time,
        ));
    }

    // 7. Add the facts to the in-memory graph
    let facts_count = facts_to_add.len();
    if facts_count > 0 {
        let mut kg = app_state
            .knowledge_graph
            .write()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to acquire KG write lock")))?;

        for (subject, predicate, object, start_time, end_time) in facts_to_add {
            kg.add_fact(&subject, &predicate, &object, start_time, end_time)
                .map_err(anyhow::Error::from)?;
        }
    }
    info!("Successfully added {facts_count} facts to the Knowledge Graph.");

    // 8. Return the response
    let response = GraphBuildResponse {
        message: "Knowledge Graph build process completed.".to_string(),
        facts_added: facts_count,
    };
    let debug_info = json!({
        "db": payload.db,
        "table_name": payload.table_name,
        "retrieved_schema": schema_string,
        "ai_mapping": {
            "subject_column": subject_column,
            "predicate_name": predicate_name,
            "object_column": object_column,
        },
        "dynamic_query": dynamic_sql,
    });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}
