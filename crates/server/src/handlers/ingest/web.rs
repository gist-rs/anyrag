use crate::auth::middleware::AuthenticatedUser;
use crate::handlers::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use anyrag::ingest::Ingestor;
use anyrag_web::{IngestionPrompts, WebIngestStrategy, WebIngestor};
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

#[derive(Deserialize)]
pub struct IngestWebRequest {
    pub url: String,
}

#[derive(Serialize)]
pub struct IngestWebResponse {
    pub message: String,
    pub ingested_faqs: usize,
}

/// Handler for the knowledge base ingestion pipeline from a web URL.
pub async fn ingest_web_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<IngestWebRequest>,
) -> Result<Json<ApiResponse<IngestWebResponse>>, AppError> {
    let owner_id = Some(user.0.id);
    info!(
        "Received web ingest request for URL: {} by user {:?}",
        payload.url, owner_id
    );

    // 1. Get necessary providers and prompts from app state
    let task_name = "knowledge_distillation";
    let task_config = app_state.tasks.get(task_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("Task '{}' not found in config", task_name))
    })?;
    let provider_name = &task_config.provider;
    let ai_provider = app_state.ai_providers.get(provider_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("Provider '{}' not found", provider_name))
    })?;

    let meta_task_name = "knowledge_metadata_extraction";
    let meta_task_config = app_state.tasks.get(meta_task_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "Task '{}' not found in config",
            meta_task_name
        ))
    })?;

    let prompts = IngestionPrompts {
        restructuring_system_prompt: &task_config.system_prompt,
        metadata_extraction_system_prompt: &meta_task_config.system_prompt,
    };

    // 2. Instantiate the ingestor plugin
    let ingestor = WebIngestor::new(&app_state.sqlite_provider.db, ai_provider.as_ref(), prompts);

    // 3. Determine the strategy and serialize the source for the ingestor
    let web_ingest_strategy = match app_state.config.web_ingest_strategy.as_str() {
        "jina" => WebIngestStrategy::Jina {
            api_key: app_state.config.jina_api_key.as_deref(),
        },
        _ => WebIngestStrategy::RawHtml,
    };

    let source_json = json!({
        "url": payload.url,
        "strategy": web_ingest_strategy,
    })
    .to_string();

    // 4. Call the generic ingest method
    let ingest_result = ingestor
        .ingest(&source_json, owner_id.as_deref())
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Web ingestion failed: {e}")))?;

    // 5. Construct the response
    let response = IngestWebResponse {
        message: "Knowledge ingestion pipeline completed successfully.".to_string(),
        ingested_faqs: ingest_result.documents_added,
    };
    let debug_info = json!({ "url": payload.url, "owner_id": owner_id });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}
