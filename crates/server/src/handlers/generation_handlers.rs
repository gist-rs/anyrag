//! # Generation Route Handlers
//!
//! This module contains handlers for endpoints that generate new content
//! based on context from the database.

use super::{wrap_response, ApiResponse, AppError, AppState, DebugParams, PromptResponse};
use crate::auth::middleware::AuthenticatedUser;
use anyrag::{
    providers::db::sqlite::SqliteProvider, types::ExecutePromptOptions, PromptClientBuilder,
};
use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::info;

// --- API Payloads for Generation Handlers ---

#[derive(Deserialize, Debug)]
pub struct GenTextRequest {
    pub db: String,
    pub generation_prompt: String,
    pub context_prompt: String,
}

// --- Generation Handlers ---

/// Handler for the advanced content generation endpoint.
///
/// This handler follows a two-stage process:
/// 1. It treats the `context_prompt` as a natural language query and executes it
///    against the specified database (`db`) to get a structured data result.
/// 2. It then uses this data result as context for the `generation_prompt`,
///    instructing an AI to generate new text based on the user's goal and the
///    retrieved data.
pub async fn gen_text_handler(
    State(app_state): State<AppState>,
    _user: AuthenticatedUser, // The user's ownership is implicitly handled by scoping to their DB.
    debug_params: Query<DebugParams>,
    Json(payload): Json<GenTextRequest>,
) -> Result<Json<ApiResponse<PromptResponse>>, AppError> {
    info!("Received text generation request for db: '{}'", payload.db);

    // --- 1. Context Retrieval via Text-to-SQL ---
    // This step executes the `context_prompt` against the specified project database.
    let context_task_name = "query_generation";
    let context_task_config = app_state
        .config
        .tasks
        .get(context_task_name)
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Task '{context_task_name}' not found in config"
            ))
        })?;
    let context_provider_name = &context_task_config.provider;
    let context_provider = app_state
        .ai_providers
        .get(context_provider_name)
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Provider '{context_provider_name}' not found"
            ))
        })?;

    // Create a dynamic SQLite client for the specified project DB.
    let db_path = format!("db/{}.db", payload.db);
    let sqlite_provider = SqliteProvider::new(&db_path).await?;
    let context_client = PromptClientBuilder::new()
        .ai_provider(context_provider.clone())
        .storage_provider(Box::new(sqlite_provider))
        .build()?;

    // The user's `context_prompt` is treated as a full-fledged prompt for the text-to-SQL engine.
    // The model should infer the table name from the prompt, so we don't specify it.
    // **Crucially**, we apply the prompt templates from our configuration to this internal call.
    let context_options = ExecutePromptOptions {
        prompt: payload.context_prompt.clone(),
        db: Some(payload.db.clone()),
        system_prompt_template: Some(context_task_config.system_prompt.clone()),
        user_prompt_template: Some(context_task_config.user_prompt.clone()),
        ..Default::default()
    };

    let context_result = context_client
        .execute_prompt_with_options(context_options)
        .await?;

    let retrieved_context = context_result.database_result.ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "The context prompt did not produce a database result. Prompt: '{}'",
            payload.context_prompt
        ))
    })?;

    if retrieved_context.trim() == "[]" {
        return Err(AppError::Internal(anyhow::anyhow!(
            "Could not find any context for the prompt '{}'. The query returned no results.",
            payload.context_prompt
        )));
    }
    info!("--> Retrieved context from DB:\n{}", retrieved_context);

    // --- 2. Content Generation ---
    let gen_task_name = "direct_generation";
    let gen_task_config = app_state.config.tasks.get(gen_task_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "Configuration for task '{gen_task_name}' not found."
        ))
    })?;
    let gen_provider_name = &gen_task_config.provider;
    let generation_provider =
        app_state.ai_providers.get(gen_provider_name).ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Provider '{gen_provider_name}' for task '{gen_task_name}' not found in providers map."
            ))
        })?;

    // Construct the final prompt for the generation provider.
    let final_user_prompt = format!(
        "# User's Goal\n{}\n\n# Inspirational Context\nDraw inspiration from the following JSON data of real online posts but don't copying directly\n---\n{}",
        payload.generation_prompt,
        retrieved_context
    );
    info!(
        "--> Sending final prompt for generation:\n{}",
        final_user_prompt
    );

    let raw_response = generation_provider
        .generate(&gen_task_config.system_prompt, &final_user_prompt)
        .await?;

    // Clean the response and attempt to parse it as JSON.
    let cleaned_response = raw_response
        .trim()
        .strip_prefix("```json")
        .unwrap_or(&raw_response)
        .strip_suffix("```")
        .unwrap_or(&raw_response)
        .trim();

    let final_value = match serde_json::from_str(cleaned_response) {
        Ok(json_value) => json_value,
        Err(_) => Value::String(raw_response.clone()), // Fallback to the original string
    };

    let debug_info = json!({
        "db": payload.db,
        "generation_prompt": payload.generation_prompt,
        "context_prompt": payload.context_prompt,
        "generated_sql_for_context": context_result.generated_sql,
        "retrieved_context": retrieved_context,
        "final_prompt_sent_to_ai": final_user_prompt,
        "raw_ai_response": raw_response,
    });

    Ok(wrap_response(
        PromptResponse { text: final_value },
        debug_params,
        Some(debug_info),
    ))
}
