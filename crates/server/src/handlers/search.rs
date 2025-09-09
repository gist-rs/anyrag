//! # Search Route Handlers
//!
//! This module contains all the Axum handlers for search-related endpoints,
//! including vector, keyword, and hybrid search.

use super::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use crate::auth::middleware::AuthenticatedUser;
use anyrag::{
    providers::{
        ai::generate_embedding,
        db::storage::{KeywordSearch, VectorSearch},
    },
    rerank::{llm_rerank, reciprocal_rank_fusion},
    search::SearchMode,
    SearchResult,
};
use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use tracing::info;

// --- API Payloads for Search ---

#[derive(Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub limit: Option<u32>,
    pub instruction: Option<String>,
    #[serde(default)]
    pub mode: SearchMode,
    #[serde(default)]
    pub use_knowledge_graph: Option<bool>,
}

// --- Search Handlers ---

/// Handler for performing a vector similarity search.
pub async fn vector_search_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<ApiResponse<Vec<SearchResult>>>, AppError> {
    let owner_id = Some(user.0.id);
    info!("Received vector search for query: '{}'", payload.query);
    let limit = payload.limit.unwrap_or(10);

    let api_url = &app_state.config.embedding.api_url;
    let model = &app_state.config.embedding.model_name;

    let query_vector = generate_embedding(api_url, model, &payload.query).await?;
    let results = app_state
        .sqlite_provider
        .vector_search(query_vector, limit, owner_id.as_deref(), None)
        .await?;

    let debug_info = json!({ "query": payload.query, "limit": limit, "owner_id": owner_id });
    Ok(wrap_response(results, debug_params, Some(debug_info)))
}

/// Handler for performing a keyword search.
pub async fn keyword_search_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<ApiResponse<Vec<SearchResult>>>, AppError> {
    let owner_id = Some(user.0.id);
    info!("Received keyword search for query: '{}'", payload.query);
    let limit = payload.limit.unwrap_or(10);
    let results = app_state
        .sqlite_provider
        .keyword_search(&payload.query, limit, owner_id.as_deref())
        .await?;
    let debug_info = json!({ "query": payload.query, "limit": limit, "owner_id": owner_id });
    Ok(wrap_response(results, debug_params, Some(debug_info)))
}

/// Handler for performing a hybrid search (vector + keyword) with re-ranking.
pub async fn hybrid_search_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<ApiResponse<Vec<SearchResult>>>, AppError> {
    let owner_id = Some(user.0.id);
    info!(
        "Received hybrid search for query: '{}' with mode {:?}",
        payload.query, payload.mode
    );
    let limit = payload.limit.unwrap_or(10);

    let api_url = &app_state.config.embedding.api_url;
    let model = &app_state.config.embedding.model_name;

    let query_vector = generate_embedding(api_url, model, &payload.query).await?;

    // --- Stage 1: Fetch Candidates Concurrently ---
    let (vector_results, keyword_results) = tokio::join!(
        app_state.sqlite_provider.vector_search(
            query_vector.clone(),
            limit * 2,
            owner_id.as_deref(),
            None
        ),
        app_state
            .sqlite_provider
            .keyword_search(&payload.query, limit * 2, owner_id.as_deref())
    );

    let vector_results = vector_results?;
    let keyword_results = keyword_results?;

    // --- Stage 2: Re-rank using the specified mode ---
    let mut ranked_results = match payload.mode {
        SearchMode::LlmReRank => {
            let mut all_candidates: HashMap<String, SearchResult> = HashMap::new();
            for result in vector_results
                .into_iter()
                .chain(keyword_results.into_iter())
            {
                all_candidates.entry(result.link.clone()).or_insert(result);
            }
            let candidates: Vec<SearchResult> = all_candidates.into_values().collect();

            if candidates.is_empty() {
                vec![]
            } else {
                // --- Task-based AI Provider Loading for Re-ranking ---
                let task_name = "llm_rerank";
                let task_config = app_state.tasks.get(task_name).ok_or_else(|| {
                    AppError::Internal(anyhow::anyhow!("Task '{task_name}' not found in config"))
                })?;
                let provider_name = &task_config.provider;
                let rerank_provider =
                    app_state.ai_providers.get(provider_name).ok_or_else(|| {
                        AppError::Internal(anyhow::anyhow!("Provider '{provider_name}' not found"))
                    })?;

                llm_rerank(
                    rerank_provider.as_ref(),
                    &payload.query,
                    candidates,
                    &task_config.system_prompt,
                    &task_config.user_prompt,
                )
                .await
                .map_err(|e| AppError::Internal(anyhow::anyhow!("LLM Reranking failed: {e}")))?
            }
        }
        SearchMode::Rrf => reciprocal_rank_fusion(vector_results, keyword_results),
    };

    ranked_results.truncate(limit as usize);

    let debug_info = json!({ "query": payload.query, "limit": limit, "mode": payload.mode, "owner_id": owner_id });
    Ok(wrap_response(
        ranked_results,
        debug_params,
        Some(debug_info),
    ))
}
