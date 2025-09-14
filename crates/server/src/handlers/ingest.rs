//! # Ingestion Route Handlers
//!
//! This module contains all the Axum handlers for data ingestion endpoints,
//! such as ingesting from RSS, text, files, and Google Sheets.

use super::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use crate::auth::middleware::AuthenticatedUser;
#[cfg(feature = "rss")]
use anyrag::ingest::ingest_from_url;
use anyrag::{
    github_ingest::{run_github_ingestion, search_examples, types::IngestionTask},
    ingest::{
        dump_firestore_collection, ingest_faq_from_google_sheet, ingest_from_google_sheet_url,
        knowledge::{extract_and_store_metadata, IngestionPrompts},
        run_ingestion_pipeline, run_pdf_ingestion_pipeline, sheet_url_to_export_url_and_table_name,
        text::{chunk_text, ingest_chunks_as_documents},
        DumpFirestoreOptions, PdfSyncExtractor,
    },
    SearchResult,
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use axum_extra::extract::Multipart;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, warn};
use turso::Value as TursoValue;
use uuid::Uuid;

// --- API Payloads for Ingestion ---

#[derive(Deserialize, Serialize, Debug)]
pub struct IngestParams {
    #[serde(default)]
    pub faq: bool,
    #[serde(default = "default_true")]
    pub embed: bool,
}

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
pub struct IngestWebRequest {
    pub url: String,
}

#[derive(Serialize)]
pub struct IngestWebResponse {
    pub message: String,
    pub ingested_faqs: usize,
}

#[derive(Deserialize)]
pub struct IngestSheetRequest {
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
pub struct IngestSheetResponse {
    pub message: String,
    pub ingested_rows: usize,
    pub table_name: Option<String>,
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
pub struct IngestFirebaseRequest {
    pub project_id: String,
    pub collection: String,
    #[serde(default)]
    pub incremental: bool,
    pub timestamp_field: Option<String>,
    pub limit: Option<i32>,
    pub fields: Option<Vec<String>>,
    #[serde(default)]
    pub use_graph: bool,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Serialize)]
pub struct IngestFirebaseResponse {
    pub message: String,
    pub ingested_documents: usize,
    pub documents_processed_for_metadata: usize,
    pub facts_added_to_graph: Option<usize>,
}

#[derive(Deserialize)]
pub struct IngestGitHubRequest {
    pub url: String,
    pub version: Option<String>,
}

#[derive(Serialize)]
pub struct IngestGitHubResponse {
    pub message: String,
    pub ingested_examples: usize,
}

#[derive(Deserialize)]
pub struct GetExamplesPath {
    pub repo_name: String,
    pub version: String,
}

#[derive(Serialize)]
pub struct GetExamplesResponse {
    pub content: String,
}

#[derive(Deserialize)]
pub struct SearchExamplesRequest {
    pub query: String,
    pub repos: Vec<String>,
}

#[derive(Serialize)]
pub struct SearchExamplesResponse {
    pub results: Vec<SearchResult>,
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

/// Unified handler for ingesting a Google Sheet as a generic table or as structured FAQs.
pub async fn ingest_sheet_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    Query(params): Query<IngestParams>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<IngestSheetRequest>,
) -> Result<Json<ApiResponse<IngestSheetResponse>>, AppError> {
    let owner_id = Some(user.0.id);
    info!(
        "User '{:?}' initiating Sheet ingest for URL: {} with params: faq={}, embed={}",
        owner_id, payload.url, params.faq, params.embed
    );

    let (ingested_count, table_name, message) = if params.faq {
        // --- FAQ Ingestion Path ---
        let count = ingest_faq_from_google_sheet(
            &app_state.sqlite_provider.db,
            &payload.url,
            owner_id.as_deref(),
            payload.gid.as_deref(),
            payload.skip_header,
        )
        .await?;
        (count, None, "Sheet FAQ ingestion successful".to_string())
    } else {
        // --- Generic Table Ingestion Path ---
        let (export_url, table_name) =
            sheet_url_to_export_url_and_table_name(&payload.url).map_err(anyhow::Error::from)?;
        let count =
            ingest_from_google_sheet_url(&app_state.sqlite_provider.db, &export_url, &table_name)
                .await
                .map_err(|e| {
                    AppError::Internal(anyhow::anyhow!("Generic sheet ingestion failed: {e}"))
                })?;
        (
            count,
            Some(table_name.clone()),
            format!("Generic sheet ingested successfully into table '{table_name}'."),
        )
    };

    let response = IngestSheetResponse {
        message,
        ingested_rows: ingested_count,
        table_name,
    };

    let debug_info = json!({
        "url": payload.url,
        "gid": payload.gid,
        "skip_header": payload.skip_header,
        "owner_id": owner_id,
        "params": params,
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

/// Consolidated handler for ingesting a PDF from an upload or a URL.
pub async fn ingest_pdf_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Query(ingest_params): Query<IngestParams>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<IngestWebResponse>>, AppError> {
    let owner_id = Some(user.0.id);
    let mut pdf_data: Option<Vec<u8>> = None;
    let mut source_identifier: Option<String> = None;
    let mut extractor_choice = ExtractorChoice::default();

    info!(
        "PDF ingest request received with params: faq={}, embed={}",
        ingest_params.faq, ingest_params.embed
    );

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

    if !ingest_params.faq {
        warn!(
            "'faq=false' is not fully implemented. Running full FAQ generation pipeline for now."
        );
    }
    if !ingest_params.embed {
        warn!("'embed=false' is not implemented. Embeddings will be generated.");
    }

    let task_name = "knowledge_distillation";
    let task_config = app_state.tasks.get(task_name).ok_or_else(|| {
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

    let augmentation_task_config =
        app_state
            .tasks
            .get("knowledge_augmentation")
            .ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!(
                    "Task 'knowledge_augmentation' not found in config"
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
        distillation_system_prompt: &task_config.system_prompt,
        distillation_user_prompt_template: &task_config.user_prompt,
        augmentation_system_prompt: &augmentation_task_config.system_prompt,
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
        "faq_enabled": ingest_params.faq,
        "embed_enabled": ingest_params.embed,
    });

    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

/// Handler for the knowledge base ingestion pipeline from a web URL.
pub async fn ingest_web_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<IngestWebRequest>,
) -> Result<Json<super::ApiResponse<IngestWebResponse>>, AppError> {
    let owner_id = Some(user.0.id);
    info!(
        "Received web ingest request for URL: {} by user {:?}",
        payload.url, owner_id
    );

    let task_name = "knowledge_distillation";
    let task_config = app_state.tasks.get(task_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("Task '{task_name}' not found in config"))
    })?;
    let provider_name = &task_config.provider;
    let ai_provider = app_state.ai_providers.get(provider_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("Provider '{provider_name}' not found"))
    })?;

    let aug_task_name = "knowledge_augmentation";
    let aug_task_config = app_state.tasks.get(aug_task_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "Task '{aug_task_name}' not found in config"
        ))
    })?;

    let meta_task_name = "knowledge_metadata_extraction";
    let meta_task_config = app_state.tasks.get(meta_task_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "Task '{meta_task_name}' not found in config"
        ))
    })?;

    let prompts = IngestionPrompts {
        extraction_system_prompt: &task_config.system_prompt,
        extraction_user_prompt_template: &task_config.user_prompt,
        augmentation_system_prompt: &aug_task_config.system_prompt,
        metadata_extraction_system_prompt: &meta_task_config.system_prompt,
    };

    let jina_api_key = app_state.config.jina_api_key.as_deref();

    let ingested_count = run_ingestion_pipeline(
        &app_state.sqlite_provider.db,
        ai_provider.as_ref(),
        &payload.url,
        owner_id.as_deref(),
        prompts,
        jina_api_key,
    )
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Knowledge ingestion failed: {e}")))?;

    let response = IngestWebResponse {
        message: "Knowledge ingestion pipeline completed successfully.".to_string(),
        ingested_faqs: ingested_count,
    };
    let debug_info = json!({ "url": payload.url, "owner_id": owner_id });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

/// Handler for ingesting a Firestore collection into a local project database.
#[cfg(feature = "firebase")]
use crate::handlers::graph_handlers;

#[cfg(feature = "firebase")]
pub async fn ingest_firebase_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<IngestFirebaseRequest>,
) -> Result<Json<ApiResponse<IngestFirebaseResponse>>, AppError> {
    let owner_id = Some(user.0.id.clone());
    info!(
        "Received Firestore ingest request for project: '{}', collection: '{}'",
        payload.project_id, payload.collection
    );

    let db_path = format!("db/{}.db", payload.project_id);
    let sqlite_provider = anyrag::providers::db::sqlite::SqliteProvider::new(&db_path).await?;
    sqlite_provider.initialize_schema().await?;

    let dump_options = DumpFirestoreOptions {
        project_id: &payload.project_id,
        collection: &payload.collection,
        incremental: payload.incremental,
        timestamp_field: payload.timestamp_field.as_deref(),
        limit: payload.limit,
        fields: payload.fields.as_deref(),
    };

    let ingested_count = dump_firestore_collection(&sqlite_provider, dump_options)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Firebase dump failed: {}", e)))?;

    if ingested_count == 0 {
        let response = IngestFirebaseResponse {
            message: "No new documents to ingest from Firestore.".to_string(),
            ingested_documents: 0,
            documents_processed_for_metadata: 0,
            facts_added_to_graph: None,
        };
        return Ok(wrap_response(response, debug_params, None));
    }

    let table_name = anyrag::ingest::firebase::sanitize_table_name(&payload.collection);
    let conn = sqlite_provider.db.connect()?;

    let source_url_prefix = format!("db://{}/{}%", payload.project_id, &table_name);
    conn.execute(
        "DELETE FROM documents WHERE source_url LIKE ?",
        turso::params![source_url_prefix],
    )
    .await?;
    info!(
        "Cleared old shadow documents for collection '{}' before ingestion.",
        payload.collection
    );

    let meta_task_config = app_state
        .tasks
        .get("knowledge_metadata_extraction")
        .unwrap();
    let (meta_ai_provider, _) = if let Some(model_name) = &payload.model {
        crate::providers::create_dynamic_provider(&app_state, model_name).await?
    } else {
        let provider_name = &meta_task_config.provider;
        let provider = app_state
            .ai_providers
            .get(provider_name)
            .ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!(
                    "Provider '{}' for task 'knowledge_metadata_extraction' not found in providers map.",
                    provider_name
                ))
            })?
            .clone();
        let provider_config = app_state.config.providers.get(provider_name).unwrap();
        (provider, provider_config.model_name.clone())
    };

    let all_data_sql = format!("SELECT * FROM {table_name}");
    let mut stmt = conn.prepare(&all_data_sql).await?;
    let column_names: Vec<String> = stmt
        .columns()
        .iter()
        .map(|c| c.name().to_string())
        .collect();
    let mut data_rows = stmt.query(()).await?;
    let mut documents_processed_for_metadata = 0;
    let id_col_index = column_names.iter().position(|name| name == "_id");

    let turso_value_to_string = |val: TursoValue| -> String {
        match val {
            TursoValue::Text(s) => s,
            TursoValue::Integer(i) => i.to_string(),
            TursoValue::Real(f) => f.to_string(),
            _ => "".to_string(),
        }
    };

    while let Some(row) = data_rows.next().await? {
        let mut document_content_parts = Vec::new();
        let mut title = String::new();

        let pk_val = id_col_index
            .and_then(|index| row.get_value(index).ok())
            .map(turso_value_to_string);

        let pk_val = match pk_val {
            Some(pk) if !pk.is_empty() => pk,
            _ => {
                warn!(
                    "Skipping row in table '{table_name}' due to missing or invalid primary key (_id)."
                );
                continue;
            }
        };

        for (i, name) in column_names.iter().enumerate() {
            let value_str = turso_value_to_string(row.get_value(i)?);
            if !value_str.is_empty() {
                if name.to_lowercase() == "title" {
                    title = value_str.clone();
                }
                document_content_parts.push(format!("{name}: {value_str}"));
            }
        }

        if title.is_empty() {
            title = pk_val.clone();
        }
        let document_content = document_content_parts.join("\n\n");
        let source_url = format!("db://{}/{}/{}", payload.project_id, table_name, pk_val);
        let document_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, source_url.as_bytes()).to_string();

        conn.execute(
            "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?, ?, ?, ?, ?)",
            turso::params![document_id.clone(), owner_id.clone(), source_url, title, document_content.clone()],
        )
        .await?;

        if let Err(e) = extract_and_store_metadata(
            &conn,
            meta_ai_provider.as_ref(),
            &document_id,
            owner_id.as_deref(),
            &document_content,
            &meta_task_config.system_prompt,
        )
        .await
        {
            info!("Could not extract metadata for doc {document_id}: {e}");
        }
        documents_processed_for_metadata += 1;
    }
    info!("Processed {documents_processed_for_metadata} documents for metadata extraction.");

    let mut facts_added_to_graph = None;
    if payload.use_graph {
        info!("`use_graph` is true. Triggering knowledge graph build for table '{table_name}'.");
        let graph_build_payload = graph_handlers::GraphBuildRequest {
            db: payload.project_id.clone(),
            table_name: table_name.clone(),
        };
        let graph_debug_params = Query(DebugParams {
            debug: debug_params.0.debug,
        });
        let graph_response = graph_handlers::graph_build_handler(
            State(app_state),
            user,
            graph_debug_params,
            Json(graph_build_payload),
        )
        .await?;
        facts_added_to_graph = Some(graph_response.0.result.facts_added);
    }

    let response = IngestFirebaseResponse {
        message: format!(
            "Successfully ingested and processed {ingested_count} documents from Firestore."
        ),
        ingested_documents: ingested_count,
        documents_processed_for_metadata,
        facts_added_to_graph,
    };

    let debug_info = json!({
        "project_id": payload.project_id,
        "collection": payload.collection,
        "incremental": payload.incremental,
        "timestamp_field": payload.timestamp_field,
        "limit": payload.limit,
        "fields": payload.fields,
        "use_graph": payload.use_graph,
        "generated_table_name": table_name,
    });

    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

/// Handler for ingesting code examples from a public GitHub repository.
pub async fn ingest_github_handler(
    State(app_state): State<AppState>,
    _user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<IngestGitHubRequest>,
) -> Result<Json<ApiResponse<IngestGitHubResponse>>, AppError> {
    info!("Received GitHub ingest request for URL: {}", payload.url);

    let task = IngestionTask {
        url: payload.url.clone(),
        version: payload.version.clone(),
        embedding_api_url: Some(app_state.config.embedding.api_url.clone()),
        embedding_model: Some(app_state.config.embedding.model_name.clone()),
    };

    let storage_manager =
        anyrag::github_ingest::storage::StorageManager::new("db/github_ingest").await?;

    let ingested_count = run_github_ingestion(&storage_manager, task)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("GitHub ingestion failed: {}", e)))?;

    let response = IngestGitHubResponse {
        message: "GitHub ingestion pipeline completed successfully.".to_string(),
        ingested_examples: ingested_count,
    };
    let debug_info = json!({ "url": payload.url, "version": payload.version });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

/// Handler for retrieving a consolidated Markdown file of examples for a repository.
pub async fn get_examples_handler(
    State(_app_state): State<AppState>,
    Path(path): Path<GetExamplesPath>,
    debug_params: Query<DebugParams>,
) -> Result<Json<ApiResponse<GetExamplesResponse>>, AppError> {
    info!(
        "Received request for examples for repo '{}', version '{}'",
        path.repo_name, path.version
    );

    let storage_manager =
        anyrag::github_ingest::storage::StorageManager::new("db/github_ingest").await?;

    let examples = storage_manager
        .get_examples(&path.repo_name, &path.version)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to retrieve examples: {}", e)))?;

    if examples.is_empty() {
        let response = GetExamplesResponse {
            content: format!(
                "# No examples found for repository '{}' version '{}'.",
                path.repo_name, path.version
            ),
        };
        return Ok(wrap_response(response, debug_params, None));
    }

    let markdown_content = examples
        .iter()
        .map(|ex| {
            format!(
                "## `{}`\n**Source:** `{}` (`{}`)\n\n```rust\n{}\n```\n",
                ex.example_handle, ex.source_file, ex.source_type, ex.content
            )
        })
        .collect::<Vec<String>>()
        .join("---\n");

    let response = GetExamplesResponse {
        content: markdown_content,
    };

    let debug_info = json!({ "repo_name": path.repo_name, "version": path.version, "example_count": examples.len() });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

/// Handler for the RAG search endpoint for code examples.
pub async fn search_examples_handler(
    State(app_state): State<AppState>,
    _user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<SearchExamplesRequest>,
) -> Result<Json<ApiResponse<SearchExamplesResponse>>, AppError> {
    info!(
        "Received example search request for query: '{}' in repos: {:?}",
        payload.query, payload.repos
    );

    let task_name = "query_analysis";
    let task_config = app_state.tasks.get(task_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "Configuration for task '{task_name}' not found."
        ))
    })?;
    let provider_name = &task_config.provider;
    let ai_provider = app_state
        .ai_providers
        .get(provider_name)
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Provider '{provider_name}' not found in providers map."
            ))
        })?
        .clone();
    let embedding_api_url = &app_state.config.embedding.api_url;
    let embedding_model = &app_state.config.embedding.model_name;

    let storage_manager =
        anyrag::github_ingest::storage::StorageManager::new("db/github_ingest").await?;

    let search_results = search_examples(
        &storage_manager,
        &payload.query,
        &payload.repos,
        std::sync::Arc::from(ai_provider),
        embedding_api_url,
        embedding_model,
    )
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Example search failed: {}", e)))?;

    let response = SearchExamplesResponse {
        results: search_results,
    };
    let debug_info = json!({ "query": payload.query, "repos": payload.repos });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}
