use crate::auth::middleware::AuthenticatedUser;
use crate::handlers::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use anyrag::ingest::{run_pdf_ingestion_pipeline, PdfSyncExtractor};
use axum::{
    extract::{Query, State},
    Json,
};
use axum_extra::extract::Multipart;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, warn};

use super::web::IngestWebResponse;

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExtractorChoice {
    #[default]
    Local,
    Gemini,
}

/// Consolidated handler for ingesting a PDF from an upload or a URL.
pub async fn ingest_pdf_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<IngestWebResponse>>, AppError> {
    let owner_id = Some(user.0.id);
    let mut pdf_data: Option<Vec<u8>> = None;
    let mut source_identifier: Option<String> = None;
    let mut extractor_choice = ExtractorChoice::default();

    info!("PDF ingest request received.");

    // --- 1. Get PDF data from either `file` or `url` part ---
    while let Some(field) = multipart.next_field().await.map_err(anyhow::Error::from)? {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "file" => {
                source_identifier =
                    Some(field.file_name().unwrap_or("uploaded_file.pdf").to_string());
                pdf_data = Some(field.bytes().await.map_err(anyhow::Error::from)?.to_vec());
                info!(
                    "User '{:?}' uploaded file: {}",
                    owner_id,
                    source_identifier.as_deref().unwrap()
                );
            }
            "url" => {
                let url = field.text().await.map_err(anyhow::Error::from)?;
                info!("User '{:?}' provided PDF URL: {}", owner_id, url);
                let response = reqwest::get(&url).await.map_err(|e| {
                    AppError::Internal(anyhow::anyhow!("Failed to download PDF from URL: {e}"))
                })?;

                if !response.status().is_success() {
                    return Err(AppError::Internal(anyhow::anyhow!(
                        "Failed to download PDF, received status: {}",
                        response.status()
                    )));
                }
                pdf_data = Some(
                    response
                        .bytes()
                        .await
                        .map_err(anyhow::Error::from)?
                        .to_vec(),
                );
                source_identifier = Some(
                    url.split('/')
                        .next_back()
                        .unwrap_or("downloaded.pdf")
                        .to_string(),
                );
            }
            "extractor" => {
                let extractor_str = field.text().await.map_err(anyhow::Error::from)?;
                extractor_choice =
                    serde_json::from_str(&format!("\"{extractor_str}\"")).map_err(|e| {
                        AppError::Internal(anyhow::anyhow!("Invalid extractor choice: {}", e))
                    })?;
                info!("Extractor choice set to: {:?}", extractor_choice);
            }
            _ => warn!("Ignoring unknown multipart field: {}", name),
        }
    }

    let pdf_data = pdf_data.ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "PDF data not found in request. Provide 'file' or 'url' part."
        ))
    })?;
    let source_identifier = source_identifier.ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "Could not determine source identifier for PDF."
        ))
    })?;

    let task_name = "knowledge_distillation";
    let task_config = app_state.tasks.get(task_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "Configuration for task '{}' not found.",
            task_name
        ))
    })?;
    let provider_name = &task_config.provider;
    let ai_provider = app_state.ai_providers.get(provider_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "Provider '{}' for task '{}' not found in providers map.",
            provider_name,
            task_name
        ))
    })?;

    let metadata_task_config = app_state
        .tasks
        .get("knowledge_metadata_extraction")
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Task 'knowledge_metadata_extraction' not found in config"
            ))
        })?;

    let extractor_strategy = match extractor_choice {
        ExtractorChoice::Local => PdfSyncExtractor::Local,
        ExtractorChoice::Gemini => PdfSyncExtractor::Gemini,
    };

    let prompts = anyrag::ingest::pdf::PdfIngestionPrompts {
        restructuring_system_prompt: &task_config.system_prompt,
        metadata_extraction_system_prompt: &metadata_task_config.system_prompt,
    };

    let ingested_count = run_pdf_ingestion_pipeline(
        &app_state.sqlite_provider.db,
        ai_provider.as_ref(),
        pdf_data.clone(),
        &source_identifier,
        owner_id.as_deref(),
        extractor_strategy,
        prompts,
    )
    .await?;

    let response = IngestWebResponse {
        message: "PDF ingestion pipeline completed successfully.".to_string(),
        ingested_faqs: ingested_count,
    };

    let debug_info = json!({
        "source": source_identifier,
        "size": pdf_data.len(),
        "extractor": extractor_choice,
        "owner_id": owner_id,
    });

    Ok(wrap_response(response, debug_params, Some(debug_info)))
}
