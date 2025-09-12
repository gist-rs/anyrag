//! # Generation Route Handlers
//!
//! This module contains handlers for endpoints that generate new content
//! based on context from the database. It features an intelligent agent
//! that decides the best method to retrieve context for generation.

use super::{wrap_response, ApiResponse, AppError, AppState, DebugParams, PromptResponse};
use crate::{auth::middleware::AuthenticatedUser, providers::create_dynamic_provider};
use anyrag::{
    providers::db::sqlite::SqliteProvider,
    search::{hybrid_search, HybridSearchOptions, HybridSearchPrompts},
    types::{ExecutePromptOptions as LibExecutePromptOptions, PromptClientBuilder},
};
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, info};

// --- API Payloads for Generation Handlers ---

fn default_true() -> bool {
    true
}

#[derive(Deserialize, Debug)]
pub struct GenTextRequest {
    #[serde(default)]
    pub db: Option<String>,
    pub generation_prompt: String,
    #[serde(default)]
    pub context_prompt: Option<String>,
    #[serde(default)]
    pub model: Option<String>,

    // New Control Flags
    #[serde(default)]
    pub use_sql: bool,
    #[serde(default)]
    pub use_knowledge_search: bool,
    #[serde(default = "default_true")]
    pub use_keyword_search: bool,
    #[serde(default = "default_true")]
    pub use_vector_search: bool,
    pub rerank_limit: Option<u32>,
}

#[derive(Deserialize, Serialize, Debug)]
struct AgentDecision {
    tool: String,
    query: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct DeconstructedQuery {
    search_query: String,
    #[serde(default)]
    generative_intent: String,
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
    // --- Provider Setup ---
    // Decide which database provider to use. If a `db` name is specified in the
    // payload, create a dynamic client for that project's database. Otherwise,
    // use the default provider from the application state.
    let (sqlite_provider, db_name) = if let Some(db_name_str) = payload.db.clone() {
        info!(
            "Request specified db: '{}'. Creating dynamic SQLite provider.",
            db_name_str
        );
        let db_path = format!("db/{db_name_str}.db");
        let provider = SqliteProvider::new(&db_path).await?;
        provider.initialize_schema().await?;
        (Arc::new(provider), db_name_str)
    } else {
        info!("No db specified in request. Using default SQLite provider.");
        (
            app_state.sqlite_provider.clone(),
            std::path::Path::new(&app_state.config.db_url)
                .file_stem()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("anyrag")
                .to_string(),
        )
    };

    info!(
        "Received text generation request for db: '{}' from user_id: {}",
        db_name, user.0.id
    );
    debug!(?payload, "Full generation request payload");

    let mut retrieved_context = String::new();
    let mut debug_context = json!({});
    let mut generative_intent = String::new();

    // --- Stage 1: Context Retrieval ---
    if let Some(context_prompt) = payload.context_prompt.as_ref().filter(|s| !s.is_empty()) {
        let agent_decision = if payload.use_sql || payload.use_knowledge_search {
            // --- Path 1: Explicit User-Directed Routing ---
            let (tool, query) = if payload.use_sql {
                ("text_to_sql", context_prompt.clone())
            } else {
                ("knowledge_search", context_prompt.clone())
            };
            info!("User explicitly selected tool: '{}'", tool);
            AgentDecision {
                tool: tool.to_string(),
                query,
            }
        } else {
            // --- Path 2: Implicit Agent-Based Routing ---
            info!("No explicit tool selected. Starting intelligent agent workflow.");
            // Attempt the two-step agentic process. If any step fails (e.g., due to a
            // non-JSON response from the LLM), this entire block will return an Err.
            let agent_result: Result<(DeconstructedQuery, AgentDecision), AppError> = async {
                // --- LLM Call 1: Deconstruct the prompt ---
                let deconstruct_task = app_state.tasks.get("query_deconstruction").unwrap();
                let deconstruct_provider = app_state
                    .ai_providers
                    .get(&deconstruct_task.provider)
                    .unwrap();
                let deconstruct_user_prompt = deconstruct_task
                    .user_prompt
                    .replace("{prompt}", context_prompt);
                debug!(system_prompt = %deconstruct_task.system_prompt, user_prompt = %deconstruct_user_prompt, "Sending prompt for deconstruction");
                let deconstruct_response_raw = deconstruct_provider
                    .generate(&deconstruct_task.system_prompt, &deconstruct_user_prompt)
                    .await?;

                let cleaned_deconstruct = deconstruct_response_raw
                    .trim()
                    .strip_prefix("```json")
                    .unwrap_or(&deconstruct_response_raw)
                    .strip_suffix("```")
                    .unwrap_or(&deconstruct_response_raw)
                    .trim();

                let deconstructed: DeconstructedQuery = serde_json::from_str(cleaned_deconstruct)?;

                // --- LLM Call 2: Select the tool for the search query ---
                let agent_task_config = app_state.tasks.get("context_agent").unwrap();
                let agent_provider = app_state
                    .ai_providers
                    .get(&agent_task_config.provider)
                    .unwrap();
                let agent_user_prompt = agent_task_config
                    .user_prompt
                    .replace("{prompt}", &deconstructed.search_query);
                debug!(system_prompt = %agent_task_config.system_prompt, user_prompt = %agent_user_prompt, "Sending prompt for agent tool selection");
                let agent_response_raw = agent_provider
                    .generate(&agent_task_config.system_prompt, &agent_user_prompt)
                    .await?;

                let cleaned_agent = agent_response_raw
                    .trim()
                    .strip_prefix("```json")
                    .unwrap_or(&agent_response_raw)
                    .strip_suffix("```")
                    .unwrap_or(&agent_response_raw)
                    .trim();

                let decision: AgentDecision = serde_json::from_str(cleaned_agent)?;
                Ok((deconstructed, decision))
            }
            .await;

            // Handle the result of the agentic workflow.
            match agent_result {
                Ok((deconstructed, decision)) => {
                    // Success: Use the agent's decision and store the deconstructed query for debugging.
                    debug_context["deconstructed_query"] = json!(deconstructed);
                    generative_intent = deconstructed.generative_intent;
                    if generative_intent.is_empty() {
                        generative_intent = context_prompt.clone();
                    }
                    decision
                }
                Err(e) => {
                    // Fallback: If the agent failed, default to text_to_sql with the original prompt.
                    info!(
                        "Agent workflow failed with error: {:?}. Falling back to text_to_sql.",
                        e
                    );
                    debug_context["agent_fallback_reason"] = json!(format!("{:?}", e));
                    AgentDecision {
                        tool: "text_to_sql".to_string(),
                        query: context_prompt.clone(),
                    }
                }
            }
        };

        info!(
            "Agent decided to use tool: '{}' with query: '{}' for user_id: {}",
            agent_decision.tool, agent_decision.query, user.0.id
        );
        debug_context["agent_decision"] = json!(agent_decision);

        // --- Tool Execution ---
        match agent_decision.tool.as_str() {
            "text_to_sql" => {
                let task_config = app_state.tasks.get("query_generation").unwrap();
                let provider = app_state.ai_providers.get(&task_config.provider).unwrap();
                let client = PromptClientBuilder::new()
                    .ai_provider(provider.clone())
                    .storage_provider(Box::new(sqlite_provider.as_ref().clone()))
                    .build()?;
                let options = LibExecutePromptOptions {
                    prompt: agent_decision.query,
                    table_name: Some("".to_string()),
                    ..Default::default()
                };
                let context_result = client.execute_prompt_with_options(options).await?;
                retrieved_context = context_result.database_result.unwrap_or_default();
                debug_context["generated_sql"] = json!(context_result.generated_sql);
            }
            "knowledge_search" => {
                let analysis_task_config = app_state.tasks.get("query_analysis").unwrap();
                let analysis_provider = app_state
                    .ai_providers
                    .get(&analysis_task_config.provider)
                    .unwrap();

                let search_options = HybridSearchOptions {
                    query_text: agent_decision.query,
                    owner_id: Some(user.0.id.clone()),
                    limit: payload.rerank_limit.unwrap_or(10),
                    prompts: HybridSearchPrompts {
                        analysis_system_prompt: &analysis_task_config.system_prompt,
                        analysis_user_prompt_template: &analysis_task_config.user_prompt,
                    },
                    use_keyword_search: payload.use_keyword_search,
                    use_vector_search: payload.use_vector_search,
                    embedding_api_url: &app_state.config.embedding.api_url,
                    embedding_model: &app_state.config.embedding.model_name,
                    temporal_ranking_config: None,
                };

                let search_results = hybrid_search(
                    sqlite_provider,
                    Arc::from(analysis_provider.clone()),
                    search_options,
                )
                .await?;
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
        info!("Context retrieval query returned no data. Aborting generation.");
        let response = PromptResponse {
            text: Value::String(
                "Failed to generate content: No relevant context was found for your request."
                    .to_string(),
            ),
        };
        let debug_info = json!({
            "status": "Aborted due to no context",
            "db": db_name,
            "generation_prompt": payload.generation_prompt,
            "context_prompt": payload.context_prompt,
            "context_retrieval_details": debug_context,
        });
        return Ok(wrap_response(response, debug_params, Some(debug_info)));
    } else if let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(&retrieved_context) {
        info!("Context retrieval returned {} items.", arr.len());
    }

    // --- Stage 2: Content Generation ---
    let gen_task_config = app_state.tasks.get("direct_generation").unwrap();
    let (generation_provider, model_used_name) = if let Some(model_name) = &payload.model {
        create_dynamic_provider(&app_state, model_name).await?
    } else {
        let provider_name = &gen_task_config.provider;
        let provider = app_state.ai_providers.get(provider_name).unwrap().clone();
        let provider_config = app_state.config.providers.get(provider_name).unwrap();
        (provider, provider_config.model_name.clone())
    };

    let user_goal = if !generative_intent.is_empty() {
        format!(
            "{}\n\n{}",
            payload.generation_prompt,
            generative_intent.trim()
        )
    } else {
        payload.generation_prompt.clone()
    };

    let final_user_prompt = if retrieved_context.is_empty() {
        user_goal
    } else {
        format!("# User's Goal\n{user_goal}\n\n# Inspirational Context\nDraw inspiration from the following JSON data of real online posts but do not copy directly.\n---\n{retrieved_context}")
    };

    info!(system_prompt = %gen_task_config.system_prompt, user_prompt = %final_user_prompt, "--> Sending final prompt for generation");
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
