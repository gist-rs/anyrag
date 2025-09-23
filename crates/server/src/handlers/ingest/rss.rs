use crate::auth::middleware::AuthenticatedUser;
use crate::handlers::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use anyrag::ingest::Ingestor;
use anyrag_rss::RssIngestor;
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
    pub message: String,
    pub ingested_articles: usize,
}

/// Handler for ingesting content from an RSS feed URL using the `anyrag-rss` plugin.
pub async fn ingest_rss_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<IngestRssRequest>,
) -> Result<Json<ApiResponse<IngestRssResponse>>, AppError> {
    let owner_id = Some(user.0.id);
    info!(
        "User '{:?}' initiating RSS ingest for URL: {}",
        owner_id, payload.url
    );

    // 1. Instantiate the ingestor plugin.
    let ingestor = RssIngestor::new(&app_state.sqlite_provider.db);

    // 2. Serialize the source information into a JSON string for the generic ingest method.
    let source_json = json!({ "url": payload.url }).to_string();

    // 3. Call the generic ingest method from the trait.
    let result = ingestor
        .ingest(&source_json, owner_id.as_deref())
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("RSS ingestion failed: {e}")))?;

    // 4. Construct the final HTTP response.
    let response = IngestRssResponse {
        message: format!(
            "Successfully ingested {} new articles from the RSS feed.",
            result.documents_added
        ),
        ingested_articles: result.documents_added,
    };

    let debug_info =
        json!({ "url": payload.url, "owner_id": owner_id, "ingested_ids": result.document_ids });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}
