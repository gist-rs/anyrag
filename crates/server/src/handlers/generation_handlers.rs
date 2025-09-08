//! # Generation Route Handlers
//!
//! This module contains handlers for endpoints that generate new content
//! based on context from the database.

use super::{
    wrap_response, ApiResponse, AppError, AppState, DebugParams, PromptResponse,
    ServerExecutePromptOptions,
};
use crate::auth::middleware::AuthenticatedUser;
use anyrag::{
    providers::{
        ai::{gemini::GeminiProvider, local::LocalAiProvider, AiProvider},
        db::sqlite::SqliteProvider,
    },
    types::ExecutePromptOptions as LibExecutePromptOptions,
    PromptClientBuilder,
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
    #[serde(default)]
    pub db: Option<String>,
    pub generation_prompt: String,
    #[serde(default)]
    pub context_prompt: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
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
    // Determine the database name, falling back to the default from config.
    let db_name = payload.db.clone().unwrap_or_else(|| {
        std::path::Path::new(&app_state.config.db_url)
            .file_stem()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("anyrag")
            .to_string()
    });
    info!("Received text generation request for db: '{}'", db_name);

    let mut retrieved_context = String::new();
    let mut context_sql: Option<String> = None;

    // --- 1. Context Retrieval via Text-to-SQL (Optional) ---
    if let Some(context_prompt) = payload.context_prompt.as_ref().filter(|s| !s.is_empty()) {
        info!("Context prompt provided. Retrieving context from DB.");
        let context_task_name = "query_generation";
        let context_task_config = app_state.tasks.get(context_task_name).ok_or_else(|| {
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

        let db_path = format!("db/{db_name}.db");
        let sqlite_provider = SqliteProvider::new(&db_path).await?;
        let context_client = PromptClientBuilder::new()
            .ai_provider(context_provider.clone())
            .storage_provider(Box::new(sqlite_provider))
            .build()?;

        let server_options = ServerExecutePromptOptions {
            prompt: context_prompt.clone(),
            db: Some(db_name.clone()),
            system_prompt_template: Some(context_task_config.system_prompt.clone()),
            user_prompt_template: Some(context_task_config.user_prompt.clone()),
            ..Default::default()
        };
        let context_options: LibExecutePromptOptions = server_options.into();

        let context_result = context_client
            .execute_prompt_with_options(context_options)
            .await?;

        retrieved_context = context_result.database_result.unwrap_or_default();
        context_sql = context_result.generated_sql;

        if retrieved_context.trim() == "[]" || retrieved_context.trim().is_empty() {
            info!("Context prompt executed but returned no results.");
            retrieved_context.clear(); // Ensure it's an empty string if no results found
        }
    } else {
        info!("No context_prompt provided, skipping context retrieval.");
    }

    // --- 2. Content Generation ---
    let (model_used_name, generation_provider, gen_task_config): (
        String,
        Box<dyn AiProvider>,
        crate::state::ResolvedTask,
    ) = if let Some(model_name) = &payload.model {
        // User has specified a model, override the provider.
        info!("User specified model override: '{}'", model_name);

        let provider: Box<dyn AiProvider> = if model_name.starts_with("gemini") {
            // It's a Gemini model.
            let api_key = std::env::var("AI_API_KEY").map_err(|_| {
                AppError::Internal(anyhow::anyhow!(
                    "AI_API_KEY must be set for dynamic Gemini provider"
                ))
            })?;
            let api_url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{model_name}:generateContent"
            );
            info!(
                "Dynamically configuring Gemini provider with URL: {}",
                api_url
            );
            Box::new(GeminiProvider::new(api_url, api_key)?)
        } else {
            // Default to a local provider.
            info!("Model is not a Gemini model, falling back to local provider configuration for URL.");
            // Find a provider named 'local_default' in the config to get its base URL.
            let local_provider_config = app_state
                .config
                .providers
                .get("local_default")
                .ok_or_else(|| {
                    AppError::Internal(anyhow::anyhow!(
                        "A 'local_default' provider was not found in configuration for fallback."
                    ))
                })?;

            Box::new(LocalAiProvider::new(
                local_provider_config.api_url.clone(),
                local_provider_config.api_key.clone(),
                Some(model_name.clone()), // Use the specified model name
            )?)
        };

        // We still need a task config for the prompts. Let's use the default `direct_generation` task for that.
        let task_config = app_state.tasks.get("direct_generation").ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Configuration for task 'direct_generation' not found."
            ))
        })?;

        (model_name.clone(), provider, task_config.clone())
    } else {
        // No model override, use the configured provider.
        let gen_task_name = "direct_generation";
        let gen_task_config = app_state.tasks.get(gen_task_name).ok_or_else(|| {
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

        // Get the model name from the provider's config
        let provider_config = app_state
            .config
            .providers
            .get(gen_provider_name)
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Provider config not found")))?;

        (
            provider_config.model_name.clone(),
            generation_provider.clone(),
            gen_task_config.clone(),
        )
    };

    // Construct the final prompt for the generation provider.
    let final_user_prompt = if retrieved_context.is_empty() {
        payload.generation_prompt.clone()
    } else {
        format!(
        "# User's Goal\n{}\n\n# Inspirational Context\nDraw inspiration from the following JSON data of real online posts but don't copying directly\n---\n{}",
        payload.generation_prompt,
        retrieved_context
    )
    };
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
        "db": db_name,
        "generation_prompt": payload.generation_prompt,
        "context_prompt": payload.context_prompt,
        "generated_sql_for_context": context_sql,
        "retrieved_context": retrieved_context,
        "final_prompt_sent_to_ai": final_user_prompt,
        "raw_ai_response": raw_response,
        "model_override": payload.model,
        "model_used": model_used_name,
    });

    Ok(wrap_response(
        PromptResponse { text: final_value },
        debug_params,
        Some(debug_info),
    ))
}
