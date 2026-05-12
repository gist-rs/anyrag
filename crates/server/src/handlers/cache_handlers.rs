//! Cache API endpoints for TTT feedback loop solution cache.
//!
//! All endpoints return meaningful data when the `solution-cache` feature is enabled,
//! and placeholder/empty responses otherwise.

use crate::{
    errors::AppError,
    handlers::{wrap_response, ApiResponse, DebugParams},
    state::AppState,
};
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tracing::info;

/// Response for `GET /cache/stats` — cache hit rate, entry count, domain breakdown.
#[derive(Serialize)]
pub struct CacheStatsResponse {
    pub entry_count: usize,
    pub max_entries: usize,
    pub avg_reward: f32,
    pub domain_counts: HashMap<String, usize>,
}

/// Handler for `GET /cache/stats` — cache statistics.
pub async fn cache_stats_handler(
    State(_state): State<AppState>,
    debug_params: Query<DebugParams>,
) -> Result<Json<ApiResponse<CacheStatsResponse>>, AppError> {
    info!("Cache stats requested");

    #[cfg(feature = "solution-cache")]
    {
        // TODO: Access SolutionCache from AppState when wired in.
        // Will call: state.solution_cache.stats() and map to CacheStatsResponse
        let stats = CacheStatsResponse {
            entry_count: 0,
            max_entries: 1000,
            avg_reward: 0.0,
            domain_counts: HashMap::new(),
        };
        let debug_info = json!({"feature": "solution-cache", "wired": false});
        Ok(wrap_response(stats, debug_params, Some(debug_info)))
    }

    #[cfg(not(feature = "solution-cache"))]
    {
        let stats = CacheStatsResponse {
            entry_count: 0,
            max_entries: 0,
            avg_reward: 0.0,
            domain_counts: HashMap::new(),
        };
        let debug_info = json!({"feature": "solution-cache", "enabled": false});
        Ok(wrap_response(stats, debug_params, Some(debug_info)))
    }
}

/// Request for `POST /cache/export` — export domain's cache as JSONL.
#[derive(Debug, Deserialize)]
pub struct CacheExportRequest {
    pub domain: String,
}

/// Response for `POST /cache/export` — exported training samples.
#[derive(Serialize)]
pub struct CacheExportResponse {
    pub samples: Vec<serde_json::Value>,
    pub count: usize,
    pub domain: String,
}

/// Handler for `POST /cache/export` — export domain's cache as riir-burner JSONL.
pub async fn cache_export_handler(
    State(_state): State<AppState>,
    Json(req): Json<CacheExportRequest>,
) -> Result<Json<ApiResponse<CacheExportResponse>>, AppError> {
    info!(domain = %req.domain, "Cache export requested");

    #[cfg(feature = "solution-cache")]
    {
        // TODO: Access SolutionCache from AppState when wired in.
        // Will call: state.solution_cache.export_jsonl(&req.domain)
        //            .into_iter().map(|s| serde_json::to_value(s)).collect()
        let response = CacheExportResponse {
            samples: Vec::new(),
            count: 0,
            domain: req.domain,
        };
        Ok(Json(ApiResponse {
            debug: None,
            result: response,
        }))
    }

    #[cfg(not(feature = "solution-cache"))]
    {
        let response = CacheExportResponse {
            samples: Vec::new(),
            count: 0,
            domain: req.domain,
        };
        let debug_info = json!({"enabled": false});
        Ok(Json(ApiResponse {
            debug: Some(debug_info),
            result: response,
        }))
    }
}

/// Handler for `DELETE /cache/prune` — manual prune trigger.
pub async fn cache_prune_handler(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    info!("Cache prune requested");

    #[cfg(feature = "solution-cache")]
    {
        // TODO: Access SolutionCache from AppState when wired in.
        // Will call: state.solution_cache.prune()
        Ok(Json(json!({
            "pruned": true,
            "message": "Cache prune triggered (placeholder — not yet wired to state)"
        })))
    }

    #[cfg(not(feature = "solution-cache"))]
    {
        Ok(Json(json!({
            "pruned": false,
            "reason": "solution-cache feature not enabled"
        })))
    }
}
