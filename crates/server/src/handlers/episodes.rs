//! # Episodes & Self-Improving Cycle Handlers
//!
//! This module contains handlers for recording translation episodes,
//! querying episode history, verifying compilation results, and
//! managing the self-improving cycle state machine.

use super::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use crate::auth::middleware::AuthenticatedUser;
use anyrag::{
    ingest::episodic,
    types::{CompilationResult, EpisodicStats, TranslationEpisode},
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;
use uuid::Uuid;

// --- Request / Response Types ---

#[derive(Deserialize)]
pub struct RecordEpisodeRequest {
    pub source_language: String,
    pub source_code: String,
    pub generated_rust: String,
    pub retrieved_context: Vec<anyrag::types::SearchResult>,
    pub hidden_state: Option<Vec<f64>>,
}

#[derive(Deserialize)]
pub struct VerifyEpisodeRequest {
    pub compilation_result: CompilationResult,
}

#[derive(Deserialize)]
pub struct ListEpisodesQuery {
    pub limit: Option<usize>,
    pub since: Option<String>,
    pub successful_only: Option<bool>,
}

#[derive(Serialize)]
pub struct RecordEpisodeResponse {
    pub id: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct VerifyEpisodeResponse {
    pub message: String,
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub stats: EpisodicStats,
}

#[derive(Serialize)]
pub struct CycleStatusResponse {
    pub status: anyrag::cycle::CycleStatus,
}

#[derive(Serialize)]
pub struct CycleTriggerResponse {
    pub message: String,
    pub action: Option<String>,
}

// --- Episode Handlers ---

/// POST /episodes — Record a new translation episode.
pub async fn record_episode_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<RecordEpisodeRequest>,
) -> Result<Json<ApiResponse<RecordEpisodeResponse>>, AppError> {
    let _ = user;
    let id = Uuid::now_v7().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();

    let episode = TranslationEpisode {
        id: id.clone(),
        source_language: payload.source_language,
        source_code: payload.source_code,
        generated_rust: payload.generated_rust,
        retrieved_context: payload.retrieved_context,
        hidden_state: payload.hidden_state,
        compilation_result: CompilationResult::NotCompiled,
        created_at,
    };

    let conn = app_state.sqlite_provider.db.connect()?;
    episodic::record_episode(&conn, &episode)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to record episode: {e}")))?;

    info!("Recorded episode: id={id}");

    let debug_info = json!({ "episode_id": id });
    Ok(wrap_response(
        RecordEpisodeResponse {
            id,
            message: "Episode recorded successfully".to_string(),
        },
        debug_params,
        Some(debug_info),
    ))
}

/// GET /episodes — List recorded episodes.
pub async fn list_episodes_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Query(query): Query<ListEpisodesQuery>,
) -> Result<Json<ApiResponse<Vec<TranslationEpisode>>>, AppError> {
    let _ = user;
    let limit = query.limit.unwrap_or(50);
    let conn = app_state.sqlite_provider.db.connect()?;

    // Currently only successful_episodes query is available in episodic module.
    // A general list endpoint can be added to episodic later.
    let episodes = episodic::get_successful_episodes(&conn, limit, query.since.as_deref())
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to list episodes: {e}")))?;

    info!("Listed {} episodes", episodes.len());

    let debug_info = json!({
        "limit": limit,
        "successful_only": query.successful_only.unwrap_or(false)
    });
    Ok(wrap_response(episodes, debug_params, Some(debug_info)))
}

/// GET /episodes/stats — Get episodic memory statistics.
pub async fn episode_stats_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
) -> Result<Json<ApiResponse<StatsResponse>>, AppError> {
    let _ = user;
    let conn = app_state.sqlite_provider.db.connect()?;
    let stats = episodic::get_stats(&conn)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get episode stats: {e}")))?;

    info!(
        "Episode stats: {} total, {} successful, {:.1}% success rate",
        stats.total_episodes,
        stats.successful,
        stats.success_rate * 100.0
    );

    let debug_info = json!({ "total_episodes": stats.total_episodes });
    Ok(wrap_response(
        StatsResponse { stats },
        debug_params,
        Some(debug_info),
    ))
}

/// POST /episodes/{id}/verify — Verify a compilation result for an episode.
pub async fn verify_episode_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Path(episode_id): Path<String>,
    Json(payload): Json<VerifyEpisodeRequest>,
) -> Result<Json<ApiResponse<VerifyEpisodeResponse>>, AppError> {
    let _ = user;
    let conn = app_state.sqlite_provider.db.connect()?;
    episodic::verify_episode(&conn, &episode_id, &payload.compilation_result)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to verify episode: {e}")))?;

    info!("Verified episode: id={episode_id}");

    let debug_info = json!({ "episode_id": episode_id });
    Ok(wrap_response(
        VerifyEpisodeResponse {
            message: "Episode verified successfully".to_string(),
        },
        debug_params,
        Some(debug_info),
    ))
}

// --- Cycle Handlers ---

/// GET /cycle/status — Get the current self-improving cycle status.
pub async fn cycle_status_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
) -> Result<Json<ApiResponse<CycleStatusResponse>>, AppError> {
    let conn = app_state.sqlite_provider.db.connect()?;
    let stats = episodic::get_stats(&conn)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get episode stats: {e}")))?;

    let cycle = app_state
        .cycle
        .lock()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to lock cycle mutex: {e}")))?;
    let status = cycle.status(Some(stats));

    info!("Cycle status: {:?}", status.state);

    let debug_info = json!({ "state": status.state });
    Ok(wrap_response(
        CycleStatusResponse { status },
        debug_params,
        Some(debug_info),
    ))
}

/// POST /cycle/trigger — Trigger a cycle tick to potentially advance the state machine.
pub async fn cycle_trigger_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
) -> Result<Json<ApiResponse<CycleTriggerResponse>>, AppError> {
    let conn = app_state.sqlite_provider.db.connect()?;
    let stats = episodic::get_stats(&conn)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get episode stats: {e}")))?;

    let mut cycle = app_state
        .cycle
        .lock()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to lock cycle mutex: {e}")))?;
    let action = cycle
        .tick(&stats)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Cycle tick failed: {e}")))?;

    let action_desc = action.map(|a| format!("{a:?}"));
    info!("Cycle trigger: action={:?}", action_desc);

    let debug_info = json!({ "action": action_desc });
    Ok(wrap_response(
        CycleTriggerResponse {
            message: "Cycle tick completed".to_string(),
            action: action_desc,
        },
        debug_params,
        Some(debug_info),
    ))
}
