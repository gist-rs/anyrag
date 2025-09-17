use crate::auth::middleware::AuthenticatedUser;
use crate::handlers::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use anyrag::ingest::Ingestor;
use anyrag_pdf::{PdfExtractor, PdfIngestor};
use anyrag_web::IngestionPrompts;
use axum::{
    extract::{Query, State},
    Json,
};
use axum_extra::extract::Multipart;
use serde_json::json;
use tracing::{info, warn};

use super::web::IngestWebResponse;
use base64::{engine::general_purpose, Engine as _};

// The ExtractorChoice is now defined in the `anyrag-pdf` crate as `PdfExtractor`.

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
    let mut extractor_choice = PdfExtractor::default();

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

    // --- 2. Get dependencies from app state ---
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
            "Provider '{}' not found in providers map.",
            provider_name
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

    let prompts = IngestionPrompts {
        restructuring_system_prompt: &task_config.system_prompt,
        metadata_extraction_system_prompt: &metadata_task_config.system_prompt,
    };

    // --- 3. Instantiate and call the ingestor plugin ---
    let ingestor = PdfIngestor::new(&app_state.sqlite_provider.db, ai_provider.as_ref(), prompts);
    let pdf_data_base64 = general_purpose::STANDARD.encode(&pdf_data);

    let source_json = json!({
        "source_identifier": source_identifier,
        "pdf_data_base64": pdf_data_base64,
        "extractor": extractor_choice,
    })
    .to_string();

    let ingest_result = ingestor
        .ingest(&source_json, owner_id.as_deref())
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("PDF ingestion failed: {e}")))?;

    // --- 4. Construct the response ---
    let response = IngestWebResponse {
        message: "PDF ingestion pipeline completed successfully.".to_string(),
        ingested_documents: ingest_result.documents_added,
    };

    let debug_info = json!({
        "source": source_identifier,
        "size": pdf_data.len(),
        "extractor": extractor_choice,
        "owner_id": owner_id,
    });

    Ok(wrap_response(response, debug_params, Some(debug_info)))
}
