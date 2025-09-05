//! # Ingestion Route Handlers
//!
//! This module contains all the Axum handlers for data ingestion endpoints,
//! such as ingesting from RSS, text, files, and Google Sheets.

use super::knowledge::KnowledgeIngestResponse;
use super::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use crate::auth::middleware::AuthenticatedUser;
#[cfg(feature = "rss")]
use anyrag::ingest::ingest_from_url;
use anyrag::ingest::{
    ingest_faq_from_google_sheet, run_pdf_ingestion_pipeline,
    text::{chunk_text, ingest_chunks_as_documents},
    PdfSyncExtractor,
};
use axum::{
    extract::{Query, State},
    Json,
};
use axum_extra::extract::Multipart;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

// --- API Payloads for Ingestion ---

#[cfg(feature = "rss")]
#[derive(Deserialize)]
pub struct IngestRssRequest {
    pub url: String,
}

#[cfg(feature = "rss")]
#[derive(Serialize)]
pub struct IngestRssResponse {
    message: String,
    ingested_articles: usize,
}

#[derive(Deserialize)]
pub struct IngestSheetFaqRequest {
    pub url: String,
    #[serde(default)]
    pub gid: Option<String>,
    #[serde(default = "default_true")]
    pub skip_header: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize)]
pub struct IngestSheetFaqResponse {
    pub message: String,
    pub ingested_faqs: usize,
}

#[derive(Deserialize)]
pub struct IngestTextRequest {
    pub text: String,
    #[serde(default = "default_source")]
    pub source: String,
}

fn default_source() -> String {
    "text_input".to_string()
}

#[derive(Serialize)]
pub struct IngestTextResponse {
    pub message: String,
    pub ingested_chunks: usize,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExtractorChoice {
    #[default]
    Local,
    Gemini,
}

#[derive(Deserialize)]
pub struct IngestPdfUrlRequest {
    pub url: String,
    #[serde(default)]
    pub extractor: ExtractorChoice,
}

// --- Ingestion Handlers ---

/// Handler for ingesting content from an RSS feed URL.
#[cfg(feature = "rss")]
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

/// Handler for ingesting structured FAQs from a Google Sheet.
pub async fn ingest_sheet_faq_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<IngestSheetFaqRequest>,
) -> Result<Json<ApiResponse<IngestSheetFaqResponse>>, AppError> {
    let owner_id = Some(user.0.id);
    info!(
        "User '{:?}' initiating Sheet FAQ ingest for URL: {} with gid: {:?}",
        owner_id, payload.url, payload.gid
    );
    let ingested_count = ingest_faq_from_google_sheet(
        &app_state.sqlite_provider.db,
        &payload.url,
        owner_id.as_deref(),
        payload.gid.as_deref(),
        payload.skip_header,
    )
    .await?;

    let response = IngestSheetFaqResponse {
        message: "Sheet FAQ ingestion successful".to_string(),
        ingested_faqs: ingested_count,
    };
    let debug_info = json!({
        "url": payload.url,
        "gid": payload.gid,
        "skip_header": payload.skip_header,
        "owner_id": owner_id
    });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

/// Handler for ingesting raw text content.
pub async fn ingest_text_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<IngestTextRequest>,
) -> Result<Json<ApiResponse<IngestTextResponse>>, AppError> {
    let owner_id = Some(user.0.id);
    info!(
        "User '{:?}' sending text ingest request from source: {}",
        owner_id, payload.source
    );
    let chunks = chunk_text(&payload.text)?;
    let total_chunks = chunks.len();

    let mut conn = app_state.sqlite_provider.db.connect()?;

    let new_document_ids =
        ingest_chunks_as_documents(&mut conn, chunks, &payload.source, owner_id.as_deref()).await?;
    let ingested_count = new_document_ids.len();

    let message = if ingested_count > 0 {
        format!("Text ingestion successful. Stored {ingested_count} new document chunks.",)
    } else if total_chunks > 0 {
        "All content may already exist. No new chunks were ingested.".to_string()
    } else {
        "No text chunks found to ingest.".to_string()
    };

    let response = IngestTextResponse {
        message,
        ingested_chunks: ingested_count,
    };
    let debug_info = json!({
        "source": payload.source,
        "chunks_created": ingested_count,
        "original_text_length": payload.text.len(),
        "document_ids": new_document_ids,
        "owner_id": owner_id,
    });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

/// Handler for ingesting a file (e.g., PDF) via multipart form data.
pub async fn ingest_file_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<KnowledgeIngestResponse>>, AppError> {
    let owner_id = Some(user.0.id);
    let mut pdf_data: Option<Vec<u8>> = None;
    let mut source_identifier: Option<String> = None;
    let mut extractor_choice = ExtractorChoice::default();

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
            "extractor" => {
                let extractor_str = field.text().await.map_err(anyhow::Error::from)?;
                extractor_choice =
                    serde_json::from_str(&format!("\"{extractor_str}\"")).map_err(|e| {
                        AppError::Internal(anyhow::anyhow!("Invalid extractor choice: {}", e))
                    })?;
                info!("Extractor choice set to: {:?}", extractor_choice);
            }
            _ => {}
        }
    }

    let pdf_data = pdf_data
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("File data not found in request.")))?;
    let source_identifier = source_identifier
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("File name not found in request.")))?;

    // --- Task-based AI Provider Loading ---
    let task_name = "knowledge_distillation";
    let task_config = app_state.config.tasks.get(task_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "Configuration for task '{task_name}' not found."
        ))
    })?;
    let provider_name = &task_config.provider;
    let ai_provider = app_state.ai_providers.get(provider_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "Provider '{provider_name}' for task '{task_name}' not found in providers map."
        ))
    })?;

    let augmentation_task_config = app_state
        .config
        .tasks
        .get("knowledge_augmentation")
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Task 'knowledge_augmentation' not found in config"
            ))
        })?;

    let extractor_strategy = match extractor_choice {
        ExtractorChoice::Local => PdfSyncExtractor::Local,
        ExtractorChoice::Gemini => PdfSyncExtractor::Gemini,
    };

    let prompts = anyrag::ingest::pdf::PdfIngestionPrompts {
        distillation_system_prompt: &task_config.system_prompt,
        distillation_user_prompt_template: &task_config.user_prompt,
        augmentation_system_prompt: &augmentation_task_config.system_prompt,
    };

    let ingested_count = run_pdf_ingestion_pipeline(
        &app_state.sqlite_provider.db,
        ai_provider.as_ref(), // Pass the dynamically selected provider
        pdf_data.clone(),
        &source_identifier,
        owner_id.as_deref(),
        extractor_strategy,
        prompts,
    )
    .await?;

    let response = KnowledgeIngestResponse {
        message: "PDF ingestion pipeline completed successfully.".to_string(),
        ingested_faqs: ingested_count,
    };

    let debug_info = json!({
        "filename": source_identifier,
        "size": pdf_data.len(),
        "extractor": extractor_choice,
        "owner_id": owner_id,
    });

    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

/// Handler for ingesting a PDF from a direct URL.
pub async fn ingest_pdf_url_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<IngestPdfUrlRequest>,
) -> Result<Json<ApiResponse<KnowledgeIngestResponse>>, AppError> {
    let owner_id = Some(user.0.id);
    info!(
        "User '{:?}' requesting PDF ingest from URL: {}",
        owner_id, payload.url
    );

    let response = reqwest::get(&payload.url)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to download PDF from URL: {e}")))?;

    if !response.status().is_success() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "Failed to download PDF, received status: {}",
            response.status()
        )));
    }
    let pdf_data = response
        .bytes()
        .await
        .map_err(anyhow::Error::from)?
        .to_vec();

    let source_identifier = payload
        .url
        .split('/')
        .next_back()
        .unwrap_or("downloaded.pdf")
        .to_string();

    info!(
        "PDF downloaded successfully. Size: {} bytes. Identifier: {}",
        pdf_data.len(),
        source_identifier
    );

    // --- Task-based AI Provider Loading ---
    let task_name = "knowledge_distillation";
    let task_config = app_state.config.tasks.get(task_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "Configuration for task '{task_name}' not found."
        ))
    })?;
    let provider_name = &task_config.provider;
    let ai_provider = app_state.ai_providers.get(provider_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "Provider '{provider_name}' for task '{task_name}' not found in providers map."
        ))
    })?;

    let augmentation_task_config = app_state
        .config
        .tasks
        .get("knowledge_augmentation")
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Task 'knowledge_augmentation' not found in config"
            ))
        })?;

    let extractor_strategy = match payload.extractor {
        ExtractorChoice::Local => PdfSyncExtractor::Local,
        ExtractorChoice::Gemini => PdfSyncExtractor::Gemini,
    };

    let prompts = anyrag::ingest::pdf::PdfIngestionPrompts {
        distillation_system_prompt: &task_config.system_prompt,
        distillation_user_prompt_template: &task_config.user_prompt,
        augmentation_system_prompt: &augmentation_task_config.system_prompt,
    };

    let ingested_count = run_pdf_ingestion_pipeline(
        &app_state.sqlite_provider.db,
        ai_provider.as_ref(), // Pass the dynamically selected provider
        pdf_data.clone(),
        &source_identifier,
        owner_id.as_deref(),
        extractor_strategy,
        prompts,
    )
    .await?;

    let response = KnowledgeIngestResponse {
        message: "PDF URL ingestion pipeline completed successfully.".to_string(),
        ingested_faqs: ingested_count,
    };

    let debug_info = json!({
        "url": payload.url,
        "filename": source_identifier,
        "size": pdf_data.len(),
        "extractor": payload.extractor,
        "owner_id": owner_id,
    });

    Ok(wrap_response(response, debug_params, Some(debug_info)))
}
