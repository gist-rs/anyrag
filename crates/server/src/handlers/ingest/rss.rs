use crate::auth::middleware::AuthenticatedUser;
use crate::handlers::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use anyrag::ingest::ingest_from_url;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

#[derive(Deserialize)]
pub struct IngestRssRequest {
    pub url: String,
}

#[derive(Serialize)]
pub struct IngestRssResponse {
    message: String,
    ingested_articles: usize,
}

/// Handler for ingesting content from an RSS feed URL.
pub async fn ingest_rss_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<IngestRssRequest>,
) -> Result<Json<ApiResponse<IngestRssResponse>>, AppError> {
    let owner_id = Some(user.0.id);
    info!(
        "User '{:?}' initiating ingest for URL: {}",
        owner_id, payload.url
    );
    let ingested_count = ingest_from_url(
        &app_state.sqlite_provider.db,
        &payload.url,
        owner_id.as_deref(),
    )
    .await?;
    let response = IngestRssResponse {
        message: "Ingestion successful".to_string(),
        ingested_articles: ingested_count,
    };
    let debug_info = json!({ "url": payload.url, "owner_id": owner_id });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}
