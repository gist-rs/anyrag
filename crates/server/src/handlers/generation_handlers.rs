//! # Generation Route Handlers
//!
//! This module contains handlers for endpoints that generate new content
//! based on context from the database. It features an intelligent agent
//! that decides the best method to retrieve context for generation.

use super::{wrap_response, ApiResponse, AppError, AppState, DebugParams, PromptResponse};
use crate::{auth::middleware::AuthenticatedUser, providers::create_dynamic_provider};
use anyrag::{
    providers::{ai::generate_embedding, db::sqlite::SqliteProvider},
    search::{hybrid_search, HybridSearchPrompts},
    types::{ExecutePromptOptions as LibExecutePromptOptions, PromptClientBuilder},
};
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
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

#[derive(Deserialize, Serialize, Debug)]
struct AgentDecision {
    tool: String,
    query: String,
}

// --- Generation Handlers ---

/// Handler for the advanced content generation endpoint.
///
/// This handler implements a two-stage agentic workflow:
/// 1.  **Context Retrieval**: If a `context_prompt` is provided, an "agent" LLM call
///     decides which tool (`text_to_sql` or `knowledge_search`) is best suited to
///     retrieve relevant data from the specified database (`db`).
/// 2.  **Content Generation**: The retrieved data is used as context for a second
///     LLM call, which uses the `generation_prompt` to create the final text output.
pub async fn gen_text_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<GenTextRequest>,
) -> Result<Json<ApiResponse<PromptResponse>>, AppError> {
    let db_name = payload.db.clone().unwrap_or_else(|| {
        std::path::Path::new(&app_state.config.db_url)
            .file_stem()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("anyrag")
            .to_string()
    });
    info!("Received text generation request for db: '{}'", db_name);

    let mut retrieved_context = String::new();
    let mut debug_context = json!({});

    // --- Stage 1: Context Retrieval via Agent ---
    if let Some(context_prompt) = payload.context_prompt.as_ref().filter(|s| !s.is_empty()) {
        info!("Context prompt provided. Executing context agent.");

        // --- LLM Call 1: Agent Tool Selection ---
        let agent_task_config = app_state
            .tasks
            .get("context_agent")
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Task 'context_agent' not found")))?;
        let agent_provider = app_state
            .ai_providers
            .get(&agent_task_config.provider)
            .ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!(
                    "Provider '{}' for agent not found",
                    agent_task_config.provider
                ))
            })?;

        let agent_user_prompt = agent_task_config
            .user_prompt
            .replace("{prompt}", context_prompt);
        let agent_response_raw = agent_provider
            .generate(&agent_task_config.system_prompt, &agent_user_prompt)
            .await?;

        let agent_response_str = agent_response_raw
            .trim()
            .strip_prefix("```json")
            .unwrap_or(&agent_response_raw)
            .strip_suffix("```")
            .unwrap_or(&agent_response_raw)
            .trim();

        let agent_decision: AgentDecision =
            serde_json::from_str(agent_response_str).map_err(|e| {
                AppError::Internal(anyhow::anyhow!(
                    "Failed to parse agent decision JSON: {}. Raw: '{}'",
                    e,
                    agent_response_raw
                ))
            })?;

        info!(
            "Agent decided to use tool: '{}' with query: '{}'",
            agent_decision.tool, agent_decision.query
        );
        debug_context["agent_decision"] = json!(agent_decision);

        // --- Tool Execution ---
        match agent_decision.tool.as_str() {
            "text_to_sql" => {
                let task_config = app_state.tasks.get("query_generation").unwrap();
                let provider = app_state.ai_providers.get(&task_config.provider).unwrap();
                let db_path = format!("db/{db_name}.db");
                let sqlite_provider = SqliteProvider::new(&db_path).await?;
                sqlite_provider.initialize_schema().await?;
                let client = PromptClientBuilder::new()
                    .ai_provider(provider.clone())
                    .storage_provider(Box::new(sqlite_provider))
                    .build()?;

                let options = LibExecutePromptOptions {
                    prompt: agent_decision.query,
                    table_name: Some("".to_string()), // Hack to fetch all schemas
                    ..Default::default()
                };

                let context_result = client.execute_prompt_with_options(options).await?;
                retrieved_context = context_result.database_result.unwrap_or_default();
                debug_context["generated_sql"] = json!(context_result.generated_sql);
            }
            "knowledge_search" => {
                let api_url = &app_state.config.embedding.api_url;
                let model = &app_state.config.embedding.model_name;
                let query_vector =
                    generate_embedding(api_url, model, &agent_decision.query).await?;

                let analysis_task_config = app_state.tasks.get("query_analysis").unwrap();
                let analysis_provider = app_state
                    .ai_providers
                    .get(&analysis_task_config.provider)
                    .unwrap();

                let db_path = format!("db/{db_name}.db");
                let sqlite_provider = SqliteProvider::new(&db_path).await?;
                sqlite_provider.initialize_schema().await?;

                let search_results = hybrid_search(
                    &sqlite_provider,
                    analysis_provider.as_ref(),
                    query_vector,
                    &agent_decision.query,
                    Some(&user.0.id),
                    10,
                    HybridSearchPrompts {
                        analysis_system_prompt: &analysis_task_config.system_prompt,
                        analysis_user_prompt_template: &analysis_task_config.user_prompt,
                    },
                )
                .await?;

                // For now, we just serialize the search results. A future improvement
                // could be to enrich this data with a second query for ratings if a
                // link between documents and original tables is established.
                retrieved_context =
                    serde_json::to_string(&search_results).map_err(anyhow::Error::from)?;
                debug_context["search_results_count"] = json!(search_results.len());
            }
            _ => {
                return Err(AppError::Internal(anyhow::anyhow!(
                    "Agent returned an unknown tool: '{}'",
                    agent_decision.tool
                )))
            }
        }
    } else {
        info!("No context_prompt provided, skipping context retrieval.");
    }

    if retrieved_context.trim() == "[]" || retrieved_context.trim().is_empty() {
        info!("Context retrieval query returned no data. Proceeding to generation without it.");
        retrieved_context.clear();
    } else if let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(&retrieved_context) {
        info!("Context retrieval returned {} items.", arr.len());
    }

    // --- Stage 2: Content Generation ---
    let gen_task_config = app_state
        .tasks
        .get("direct_generation")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Task 'direct_generation' not found")))?;

    let (generation_provider, model_used_name) = if let Some(model_name) = &payload.model {
        create_dynamic_provider(&app_state, model_name).await?
    } else {
        let provider_name = &gen_task_config.provider;
        let provider = app_state.ai_providers.get(provider_name).unwrap().clone();
        let provider_config = app_state.config.providers.get(provider_name).unwrap();
        (provider, provider_config.model_name.clone())
    };

    let final_user_prompt = if retrieved_context.is_empty() {
        payload.generation_prompt.clone()
    } else {
        format!(
            "# User's Goal\n{}\n\n# Inspirational Context\nDraw inspiration from the following JSON data of real online posts but do not copy directly.\n---\n{}",
            payload.generation_prompt,
            retrieved_context
        )
    };

    let raw_response = generation_provider
        .generate(&gen_task_config.system_prompt, &final_user_prompt)
        .await?;

    let cleaned_response = raw_response
        .trim()
        .strip_prefix("```json")
        .unwrap_or(&raw_response)
        .strip_suffix("```")
        .unwrap_or(&raw_response)
        .trim();

    let final_value = match serde_json::from_str(cleaned_response) {
        Ok(json_value) => json_value,
        Err(_) => Value::String(raw_response.clone()),
    };

    let debug_info = json!({
        "db": db_name,
        "generation_prompt": payload.generation_prompt,
        "context_prompt": payload.context_prompt,
        "context_retrieval_details": debug_context,
        "retrieved_context_summary": if retrieved_context.is_empty() {
            json!(null)
        } else {
            json!(format!("{} bytes", retrieved_context.len()))
        },
        "final_prompt_sent_to_ai_summary": format!("{} bytes", final_user_prompt.len()),
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
