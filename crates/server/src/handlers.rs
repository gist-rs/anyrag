use super::{
    errors::AppError,
    state::AppState,
    types::{ApiResponse, DebugParams, ExtractorChoice, IngestTextRequest, IngestTextResponse},
};
use anyrag::providers::db::storage::Storage;
use anyrag::{
    ingest::{
        export_for_finetuning, ingest_faq_from_google_sheet, ingest_from_google_sheet_url,
        ingest_from_url, run_ingestion_pipeline, run_pdf_ingestion_pipeline,
        sheet_url_to_export_url_and_table_name,
        text::{chunk_text, ingest_chunks_as_documents},
        PdfSyncExtractor,
    },
    providers::{
        ai::generate_embedding,
        db::storage::{KeywordSearch, VectorSearch},
    },
    search::hybrid_search,
    types::ContentType,
    ExecutePromptOptions, PromptClientBuilder, SearchResult,
};
use axum::{
    extract::{Query, State},
    Json,
};
use axum_extra::extract::Multipart;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use tracing::{error, info};
use turso::params;

// --- API Payloads ---

#[derive(Serialize, Deserialize)]
pub struct PromptResponse {
    pub text: String,
}

#[derive(Deserialize)]
pub struct IngestRequest {
    pub url: String,
}

#[derive(Serialize)]
pub struct IngestResponse {
    message: String,
    ingested_articles: usize,
}

#[derive(Serialize)]
pub struct KnowledgeIngestResponse {
    pub message: String,
    pub ingested_faqs: usize,
}

#[derive(Deserialize, Debug)]
pub struct EmbedNewRequest {
    pub limit: Option<usize>,
}

#[derive(Serialize, Debug)]
pub struct EmbedNewResponse {
    message: String,
    embedded_articles: usize,
}

#[derive(Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub limit: Option<u32>,
    pub instruction: Option<String>,

    #[serde(default)]
    pub use_knowledge_graph: Option<bool>,
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

// --- Helper Functions ---

fn wrap_response<T>(
    result: T,
    debug_params: Query<DebugParams>,
    debug_info: Option<Value>,
) -> Json<ApiResponse<T>> {
    let debug = if debug_params.debug.unwrap_or(false) {
        debug_info
    } else {
        None
    };
    Json(ApiResponse { debug, result })
}

// --- Route Handlers ---

pub async fn root() -> &'static str {
    "anyrag server is running."
}

pub async fn health_check() -> &'static str {
    "OK"
}

pub async fn prompt_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<Value>,
) -> Result<Json<ApiResponse<PromptResponse>>, AppError> {
    info!("Received prompt payload: '{}'", payload);
    let mut options: ExecutePromptOptions =
        serde_json::from_value(payload).map_err(anyrag::PromptError::from)?;

    if options.system_prompt_template.is_none() {
        options.system_prompt_template = app_state.query_system_prompt_template.clone();
    }
    if options.user_prompt_template.is_none() {
        options.user_prompt_template = app_state.query_user_prompt_template.clone();
    }
    if options.format_system_prompt_template.is_none() {
        options.format_system_prompt_template = app_state.format_system_prompt_template.clone();
    }
    if options.format_user_prompt_template.is_none() {
        options.format_user_prompt_template = app_state.format_user_prompt_template.clone();
    }

    let sheet_url = options
        .prompt
        .split_whitespace()
        .find(|word| word.contains("/spreadsheets/d/"));

    let prompt_result = if let Some(url) = sheet_url {
        info!("Detected Google Sheet URL in prompt: {}", url);
        let (export_url, table_name) = sheet_url_to_export_url_and_table_name(url)
            .map_err(|e| anyhow::anyhow!("Sheet URL transformation failed: {e}"))?;

        if app_state
            .sqlite_provider
            .get_table_schema(&table_name)
            .await
            .is_err()
        {
            info!("Table '{table_name}' does not exist. Starting ingestion.");
            ingest_from_google_sheet_url(&app_state.sqlite_provider.db, &export_url, &table_name)
                .await
                .map_err(|e| anyhow::anyhow!("Sheet ingestion failed: {e}"))?;
        } else {
            info!("Table '{table_name}' already exists. Skipping ingestion.");
        }

        options.table_name = Some(table_name);
        let sqlite_prompt_client = PromptClientBuilder::new()
            .ai_provider(app_state.prompt_client.ai_provider.clone())
            .storage_provider(Box::new(app_state.sqlite_provider.as_ref().clone()))
            .build()?;
        sqlite_prompt_client
            .execute_prompt_with_options(options.clone())
            .await?
    } else {
        app_state
            .prompt_client
            .execute_prompt_with_options(options.clone())
            .await?
    };

    let debug_info = if debug_params.debug.unwrap_or(false) {
        Some(json!({
            "options": options,
            "generated_sql": prompt_result.generated_sql,
            "database_result": prompt_result.database_result,
        }))
    } else {
        None
    };
    Ok(wrap_response(
        PromptResponse {
            text: prompt_result.text,
        },
        debug_params,
        debug_info,
    ))
}

pub async fn ingest_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<IngestRequest>,
) -> Result<Json<ApiResponse<IngestResponse>>, AppError> {
    info!("Received ingest request for URL: {}", payload.url);
    let ingested_count = ingest_from_url(&app_state.sqlite_provider.db, &payload.url).await?;
    let response = IngestResponse {
        message: "Ingestion successful".to_string(),
        ingested_articles: ingested_count,
    };
    let debug_info = json!({ "url": payload.url });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

pub async fn ingest_sheet_faq_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<IngestSheetFaqRequest>,
) -> Result<Json<ApiResponse<IngestSheetFaqResponse>>, AppError> {
    info!(
        "Received Sheet FAQ ingest request for URL: {} with gid: {:?}",
        payload.url, payload.gid
    );
    let ingested_count = ingest_faq_from_google_sheet(
        &app_state.sqlite_provider.db,
        &payload.url,
        payload.gid.as_deref(),
        payload.skip_header,
    )
    .await?;

    let response = IngestSheetFaqResponse {
        message: "Sheet FAQ ingestion successful".to_string(),
        ingested_faqs: ingested_count,
    };
    let debug_info =
        json!({ "url": payload.url, "gid": payload.gid, "skip_header": payload.skip_header });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

pub async fn ingest_text_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<IngestTextRequest>,
) -> Result<Json<ApiResponse<IngestTextResponse>>, AppError> {
    info!(
        "Received text ingest request from source: {}",
        payload.source
    );
    let chunks = chunk_text(&payload.text)?;
    let total_chunks = chunks.len();

    let mut conn = app_state.sqlite_provider.db.connect()?;

    // TODO: Add owner_id from JWT once auth is implemented.
    let new_document_ids =
        ingest_chunks_as_documents(&mut conn, chunks, &payload.source, None).await?;
    let ingested_count = new_document_ids.len();

    // TODO: Re-implement auto-embedding for the new `documents` schema.
    // This will likely involve a new `embed_document` function and a background job.

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
    });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

pub async fn ingest_file_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<KnowledgeIngestResponse>>, AppError> {
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
                    "Received file upload: {}",
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
            _ => {
                // Ignore other fields
            }
        }
    }

    let pdf_data = pdf_data
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("File data not found in request.")))?;
    let source_identifier = source_identifier
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("File name not found in request.")))?;

    let extractor_strategy = match extractor_choice {
        ExtractorChoice::Local => PdfSyncExtractor::Local,
        ExtractorChoice::Gemini => PdfSyncExtractor::Gemini,
    };

    let ingested_count = run_pdf_ingestion_pipeline(
        &app_state.sqlite_provider.db,
        &*app_state.prompt_client.ai_provider,
        pdf_data.clone(),
        &source_identifier,
        extractor_strategy,
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
    });

    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

#[derive(Deserialize)]
pub struct IngestPdfUrlRequest {
    pub url: String,
    #[serde(default)]
    pub extractor: ExtractorChoice,
}

pub async fn ingest_pdf_url_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<IngestPdfUrlRequest>,
) -> Result<Json<ApiResponse<KnowledgeIngestResponse>>, AppError> {
    info!("Received PDF ingest request for URL: {}", payload.url);

    // 1. Download the PDF, reqwest follows redirects by default.
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

    // 2. Determine a source identifier from the URL.
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

    let extractor_strategy = match payload.extractor {
        ExtractorChoice::Local => PdfSyncExtractor::Local,
        ExtractorChoice::Gemini => PdfSyncExtractor::Gemini,
    };

    // 3. Run the existing ingestion pipeline.
    let ingested_count = run_pdf_ingestion_pipeline(
        &app_state.sqlite_provider.db,
        &*app_state.prompt_client.ai_provider,
        pdf_data.clone(),
        &source_identifier,
        extractor_strategy,
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
    });

    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

pub async fn embed_new_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<EmbedNewRequest>,
) -> Result<Json<ApiResponse<EmbedNewResponse>>, AppError> {
    let limit = payload.limit.unwrap_or(20);
    info!("Received request to embed up to {limit} new documents.");
    let api_url = app_state
        .embeddings_api_url
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("EMBEDDINGS_API_URL not set")))?
        .clone();
    let model = app_state
        .embeddings_model
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("EMBEDDINGS_MODEL not set")))?
        .clone();

    let conn = app_state.sqlite_provider.db.connect()?;
    let sql = format!(
        "
        SELECT d.id, d.title, d.content
        FROM documents d
        LEFT JOIN document_embeddings de ON d.id = de.document_id
        WHERE de.id IS NULL
        LIMIT {limit}
    "
    );
    let mut stmt = conn.prepare(&sql).await?;
    let mut rows = stmt.query(()).await?;

    let mut docs_to_embed = Vec::new();
    while let Some(row) = rows.next().await? {
        let id: String = row.get(0)?;
        let title: String = row.get(1)?;
        let content: String = row.get(2)?;
        docs_to_embed.push((id, title, content));
    }

    let embed_count = docs_to_embed.len();
    info!("Found {embed_count} documents to embed.");

    if docs_to_embed.is_empty() {
        let response = EmbedNewResponse {
            message: "No new documents to embed.".to_string(),
            embedded_articles: 0,
        };
        let debug_info = json!({ "limit": limit, "found": 0 });
        return Ok(wrap_response(response, debug_params, Some(debug_info)));
    }

    let mut embedded_ids = Vec::new();
    for (doc_id, title, content) in docs_to_embed {
        let text_to_embed = format!("{title}. {content}");
        match generate_embedding(&api_url, &model, &text_to_embed).await {
            Ok(vector) => {
                let vector_bytes: &[u8] = unsafe {
                    std::slice::from_raw_parts(vector.as_ptr() as *const u8, vector.len() * 4)
                };
                if let Err(e) = conn
                    .execute(
                        "INSERT INTO document_embeddings (document_id, model_name, embedding) VALUES (?, ?, ?)",
                        params![doc_id.clone(), model.clone(), vector_bytes],
                    )
                    .await
                {
                    error!("Failed to insert embedding for document ID: {doc_id}. Error: {e}");
                } else {
                    info!("Successfully embedded document ID: {}", doc_id);
                    embedded_ids.push(doc_id);
                }
            }
            Err(e) => {
                error!("Failed to generate embedding for document ID: {doc_id}. Error: {e}");
            }
        }
    }

    let success_count = embedded_ids.len();
    let response = EmbedNewResponse {
        message: format!(
            "Successfully processed embeddings for {success_count} of {embed_count} documents."
        ),
        embedded_articles: success_count,
    };
    let debug_info = json!({ "limit": limit, "found": embed_count, "embedded_ids": embedded_ids });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

pub async fn vector_search_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<ApiResponse<Vec<SearchResult>>>, AppError> {
    info!("Received vector search for query: '{}'", payload.query);
    let limit = payload.limit.unwrap_or(10);
    let api_url = app_state
        .embeddings_api_url
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("EMBEDDINGS_API_URL not set")))?;
    let model = app_state
        .embeddings_model
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("EMBEDDINGS_MODEL not set")))?;

    let query_vector = generate_embedding(api_url, model, &payload.query).await?;
    let results = app_state
        .sqlite_provider
        .vector_search(query_vector, limit, None)
        .await?;

    let debug_info = json!({ "query": payload.query, "limit": limit });
    Ok(wrap_response(results, debug_params, Some(debug_info)))
}

pub async fn keyword_search_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<ApiResponse<Vec<SearchResult>>>, AppError> {
    info!("Received keyword search for query: '{}'", payload.query);
    let limit = payload.limit.unwrap_or(10);
    let results = app_state
        .sqlite_provider
        .keyword_search(&payload.query, limit)
        .await?;
    let debug_info = json!({ "query": payload.query, "limit": limit });
    Ok(wrap_response(results, debug_params, Some(debug_info)))
}

pub async fn knowledge_ingest_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<IngestRequest>,
) -> Result<Json<ApiResponse<KnowledgeIngestResponse>>, AppError> {
    info!("Received knowledge ingest request for URL: {}", payload.url);
    let ingested_count = run_ingestion_pipeline(
        &app_state.sqlite_provider.db,
        &*app_state.prompt_client.ai_provider,
        &payload.url,
    )
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Knowledge ingestion failed: {e}")))?;
    let response = KnowledgeIngestResponse {
        message: "Knowledge ingestion pipeline completed successfully.".to_string(),
        ingested_faqs: ingested_count,
    };
    let debug_info = json!({ "url": payload.url });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

pub async fn knowledge_export_handler(
    State(app_state): State<AppState>,
) -> Result<String, AppError> {
    info!("Received request to export knowledge base for fine-tuning.");
    let jsonl_data = export_for_finetuning(&app_state.sqlite_provider.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Knowledge export failed: {e}")))?;
    Ok(jsonl_data)
}

pub async fn knowledge_search_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<ApiResponse<PromptResponse>>, AppError> {
    let limit = payload.limit.unwrap_or(5);
    info!(
        "Received knowledge RAG search for query: '{}', limit: {}",
        payload.query, limit
    );
    // TODO: Add owner_id from JWT once auth is implemented.

    let api_url = app_state
        .embeddings_api_url
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("EMBEDDINGS_API_URL not set")))?;
    let model = app_state
        .embeddings_model
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("EMBEDDINGS_MODEL not set")))?;

    // --- Stage 1: Get Query Embedding ---
    let query_vector = generate_embedding(api_url, model, &payload.query).await?;

    // --- Stage 2 & 3: Hybrid Search (Metadata -> Vector) ---
    let search_results = hybrid_search(
        app_state.sqlite_provider.as_ref(),
        app_state.prompt_client.ai_provider.as_ref(),
        query_vector,
        &payload.query,
        None, // owner_id
        limit,
    )
    .await?;

    // --- Stage 4: Context Aggregation ---
    // --- Stage 4: Context Aggregation ---
    let kg_fact = if payload.use_knowledge_graph.unwrap_or(false) {
        info!("Knowledge graph search is enabled for this request.");
        let kg = app_state
            .knowledge_graph
            .read()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to acquire KG read lock")))?;

        // For now, let's assume the predicate is 'role' if not specified.
        // A more advanced implementation might extract this from the query.
        let predicate = "role";
        kg.get_fact_as_of(&payload.query, predicate, Utc::now())
            .ok()
            .flatten()
    } else {
        None
    };

    let mut context_parts = Vec::new();

    if let Some(fact) = kg_fact {
        info!("Found definitive fact in Knowledge Graph: {}", fact);
        context_parts.push(format!("Definitive Answer from Knowledge Graph: {fact}."));
    }

    if !search_results.is_empty() {
        // Use the `description` which contains the full document content.
        let articles_context = search_results
            .iter()
            .map(|result| result.description.clone())
            .collect::<Vec<String>>()
            .join("\n\n---\n\n");

        if !context_parts.is_empty() {
            context_parts.push(format!(
                "Additional Context from Documents:\n{articles_context}"
            ));
        } else {
            context_parts.push(articles_context);
        }
    }

    let context = context_parts.join("\n\n");

    if context.is_empty() {
        let text = "I could not find any relevant information to answer your question.".to_string();
        let debug_info =
            json!({ "query": payload.query, "limit": limit, "status": "No results found" });
        return Ok(wrap_response(
            PromptResponse { text },
            debug_params,
            Some(debug_info),
        ));
    }

    info!("--> Synthesizing answer with context:\n{}", context);

    // --- Stage 5: LLM Synthesis ---
    let options = ExecutePromptOptions {
        prompt: payload.query.clone(),
        content_type: Some(ContentType::Knowledge),
        context: Some(context.clone()),
        instruction: payload.instruction,
        ..Default::default()
    };
    let prompt_result = app_state
        .prompt_client
        .execute_prompt_with_options(options.clone())
        .await?;

    let debug_info = if debug_params.debug.unwrap_or(false) {
        Some(json!({
            "options": options,
            "retrieved_context": context,
            "final_candidate_count": search_results.len()
        }))
    } else {
        None
    };
    Ok(wrap_response(
        PromptResponse {
            text: prompt_result.text,
        },
        debug_params,
        debug_info,
    ))
}

// --- Knowledge Graph Payloads ---

#[derive(Deserialize)]
pub struct KnowledgeGraphSearchRequest {
    pub subject: String,
    pub predicate: String,
}

#[derive(Serialize)]
pub struct KnowledgeGraphSearchResponse {
    pub object: Option<String>,
}

// --- Knowledge Graph Handler ---

pub async fn knowledge_graph_search_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<KnowledgeGraphSearchRequest>,
) -> Result<Json<ApiResponse<KnowledgeGraphSearchResponse>>, AppError> {
    info!(
        "Received knowledge graph search for subject: '{}', predicate: '{}'",
        payload.subject, payload.predicate
    );

    let object = {
        let kg = app_state
            .knowledge_graph
            .read()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to acquire KG read lock")))?;
        kg.get_fact_as_of(&payload.subject, &payload.predicate, Utc::now())
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Knowledge graph query failed: {e}")))?
    };

    let response = KnowledgeGraphSearchResponse { object };

    // For simplicity, we are not including debug info in this handler for now.
    Ok(Json(ApiResponse {
        debug: None,
        result: response,
    }))
}
