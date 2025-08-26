use super::{
    errors::AppError,
    state::AppState,
    types::{ApiResponse, DebugParams, IngestTextRequest, IngestTextResponse},
};
use anyrag::{
    ingest::{
        articles::{insert_articles, Article as IngestArticle},
        create_articles_table_if_not_exists, embed_article, embed_faq, export_for_finetuning,
        ingest_from_google_sheet_url, ingest_from_url, run_ingestion_pipeline,
        sheet_url_to_export_url_and_table_name,
        text::chunk_text,
    },
    providers::{
        ai::generate_embedding,
        db::storage::{KeywordSearch, Storage, VectorSearch},
    },
    search::{hybrid_search, SearchMode},
    types::ContentType,
    ExecutePromptOptions, PromptClientBuilder, SearchResult,
};
use axum::{
    extract::{Query, State},
    Json,
};
use futures::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::hash::{Hash, Hasher};
use tracing::{error, info, warn};
use turso::Value as TursoValue;

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

#[derive(Deserialize)]
pub struct EmbedRequest {
    article_id: i64,
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
    pub mode: SearchMode,
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

    let db = app_state.sqlite_provider.db.clone();
    let conn = db.connect()?;
    if let Err(e) = create_articles_table_if_not_exists(&conn).await {
        if !e.to_string().contains("already exists") {
            return Err(e.into());
        }
        warn!(
            "Ignoring benign 'index already exists' error during table setup: {}",
            e
        );
    }

    let articles_to_insert: Vec<IngestArticle> = chunks
        .into_iter()
        .map(|chunk| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            chunk.hash(&mut hasher);
            let hash = hasher.finish();
            IngestArticle {
                title: chunk.chars().take(80).collect(),
                link: format!("{}_{:x}", payload.source, hash),
                description: chunk,
                source_url: payload.source.clone(),
                pub_date: None,
            }
        })
        .collect();

    let new_article_ids = insert_articles(&conn, articles_to_insert).await?;
    let ingested_count = new_article_ids.len();

    if !new_article_ids.is_empty() {
        info!("Found {} new articles to embed.", new_article_ids.len());
        let api_url = app_state.embeddings_api_url.clone().ok_or_else(|| {
            anyhow::anyhow!("EMBEDDINGS_API_URL not set to auto-embed ingested text")
        })?;
        let model = app_state.embeddings_model.clone().ok_or_else(|| {
            anyhow::anyhow!("EMBEDDINGS_MODEL not set to auto-embed ingested text")
        })?;

        stream::iter(new_article_ids)
            .for_each_concurrent(10, |article_id| {
                let db_clone = db.clone();
                let api_url_clone = api_url.clone();
                let model_clone = model.clone();
                async move {
                    match embed_article(&db_clone, &api_url_clone, &model_clone, article_id).await {
                        Ok(_) => info!("Auto-embedded article ID: {}", article_id),
                        Err(e) => error!(
                            "Failed to auto-embed article ID: {}. Error: {}",
                            article_id, e
                        ),
                    }
                }
            })
            .await;
    }

    let message = if ingested_count > 0 {
        format!(
            "Text ingestion successful. Stored and embedded {} new chunks.",
            ingested_count
        )
    } else if total_chunks > 0 {
        "All content already exists. No new chunks were ingested.".to_string()
    } else {
        "No text chunks found to ingest.".to_string()
    };

    let response = IngestTextResponse {
        message,
        ingested_chunks: ingested_count,
    };
    let debug_info = json!({ "source": payload.source, "chunks_processed": ingested_count, "original_text_length": payload.text.len() });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

pub async fn embed_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<EmbedRequest>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let api_url = app_state
        .embeddings_api_url
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("EMBEDDINGS_API_URL not set")))?;
    let model = app_state
        .embeddings_model
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("EMBEDDINGS_MODEL not set")))?;

    embed_article(
        &app_state.sqlite_provider.db,
        api_url,
        model,
        payload.article_id,
    )
    .await?;
    let response = json!({ "success": true });
    let debug_info = json!({ "article_id": payload.article_id });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

pub async fn embed_new_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<EmbedNewRequest>,
) -> Result<Json<ApiResponse<EmbedNewResponse>>, AppError> {
    let limit = payload.limit.unwrap_or(10);
    info!(
        "Received request to find and embed up to {} new articles.",
        limit
    );
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
        "SELECT id FROM articles WHERE embedding IS NULL ORDER BY created_at DESC LIMIT {limit}"
    );
    let mut stmt = conn.prepare(&sql).await?;
    let mut rows = stmt.query(()).await?;

    let mut articles_to_embed = Vec::new();
    while let Some(row) = rows.next().await? {
        if let Ok(TursoValue::Integer(id)) = row.get_value(0) {
            articles_to_embed.push(id);
        }
    }
    let embed_count = articles_to_embed.len();
    info!("Found {embed_count} articles to embed.");

    if articles_to_embed.is_empty() {
        let response = EmbedNewResponse {
            message: "No new articles to embed.".to_string(),
            embedded_articles: 0,
        };
        let debug_info = json!({ "limit": limit, "found": 0 });
        return Ok(wrap_response(response, debug_params, Some(debug_info)));
    }

    let articles_to_embed_clone = articles_to_embed.clone();
    stream::iter(articles_to_embed)
        .for_each_concurrent(10, |article_id| {
            let db = app_state.sqlite_provider.db.clone();
            let api_url = api_url.clone();
            let model = model.clone();
            async move {
                match embed_article(&db, &api_url, &model, article_id).await {
                    Ok(_) => info!("Successfully embedded article ID: {article_id}"),
                    Err(e) => error!("Failed to embed article ID: {article_id}. Error: {e}"),
                }
            }
        })
        .await;

    let response = EmbedNewResponse {
        message: format!("Successfully processed {embed_count} articles."),
        embedded_articles: embed_count,
    };
    let debug_info =
        json!({ "limit": limit, "found": embed_count, "embedded_ids": articles_to_embed_clone });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

pub async fn vector_search_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<ApiResponse<Vec<SearchResult>>>, AppError> {
    let limit = payload.limit.unwrap_or(10);
    info!(
        "Received vector search request for query: '{}', limit: {}",
        payload.query, limit
    );
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
        .vector_search(query_vector, limit as u32)
        .await?;
    let debug_info = json!({ "query": payload.query, "limit": limit });
    Ok(wrap_response(results, debug_params, Some(debug_info)))
}

pub async fn keyword_search_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<ApiResponse<Vec<SearchResult>>>, AppError> {
    let limit = payload.limit.unwrap_or(10);
    info!(
        "Received keyword search request for query: '{}', limit: {}",
        payload.query, limit
    );
    let results = app_state
        .sqlite_provider
        .keyword_search(&payload.query, limit as u32)
        .await?;
    let debug_info = json!({ "query": payload.query, "limit": limit });
    Ok(wrap_response(results, debug_params, Some(debug_info)))
}

pub async fn hybrid_search_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<ApiResponse<Vec<SearchResult>>>, AppError> {
    let limit = payload.limit.unwrap_or(10);
    info!(
        "Received hybrid search request for query: '{}', limit: {}, mode: {:?}",
        payload.query, limit, payload.mode
    );
    let api_url = app_state
        .embeddings_api_url
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("EMBEDDINGS_API_URL not set")))?;
    let model = app_state
        .embeddings_model
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("EMBEDDINGS_MODEL not set")))?;
    let query_vector = generate_embedding(api_url, model, &payload.query).await?;
    let results = hybrid_search(
        app_state.sqlite_provider.as_ref(),
        &*app_state.prompt_client.ai_provider,
        query_vector,
        &payload.query,
        limit as u32,
        payload.mode,
    )
    .await?;
    let debug_info = json!({ "query": payload.query, "limit": limit, "mode": payload.mode });
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
    let api_url = app_state
        .embeddings_api_url
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("EMBEDDINGS_API_URL not set")))?;
    let model = app_state
        .embeddings_model
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("EMBEDDINGS_MODEL not set")))?;
    let query_vector = generate_embedding(api_url, model, &payload.query).await?;
    let search_results = app_state
        .sqlite_provider
        .vector_search_faqs(query_vector, limit as u32)
        .await?;

    if search_results.is_empty() {
        let text = "I could not find any relevant information to answer your question.".to_string();
        let debug_info =
            json!({ "query": payload.query, "limit": limit, "status": "No results found" });
        return Ok(wrap_response(
            PromptResponse { text },
            debug_params,
            Some(debug_info),
        ));
    }
    let context = search_results
        .iter()
        .map(|result| format!("- {}", result.answer))
        .collect::<Vec<String>>()
        .join("\n\n");
    info!("--> Synthesizing answer with context:\n{}", context);

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
        Some(json!({ "options": options, "retrieved_context": context }))
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

pub async fn embed_faqs_new_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<EmbedNewRequest>,
) -> Result<Json<ApiResponse<EmbedNewResponse>>, AppError> {
    let limit = payload.limit.unwrap_or(20);
    info!("Received request to embed up to {limit} new FAQs.");
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
    let sql = format!("SELECT id FROM faq_kb WHERE embedding IS NULL LIMIT {limit}");
    let mut stmt = conn.prepare(&sql).await?;
    let mut rows = stmt.query(()).await?;

    let mut faqs_to_embed = Vec::new();
    while let Some(row) = rows.next().await? {
        if let Ok(TursoValue::Integer(id)) = row.get_value(0) {
            faqs_to_embed.push(id);
        }
    }
    let embed_count = faqs_to_embed.len();
    info!("Found {embed_count} FAQs to embed.");

    if faqs_to_embed.is_empty() {
        let response = EmbedNewResponse {
            message: "No new FAQs to embed.".to_string(),
            embedded_articles: 0,
        };
        let debug_info = json!({ "limit": limit, "found": 0 });
        return Ok(wrap_response(response, debug_params, Some(debug_info)));
    }

    let faqs_to_embed_clone = faqs_to_embed.clone();
    stream::iter(faqs_to_embed)
        .for_each_concurrent(10, |faq_id| {
            let db = app_state.sqlite_provider.db.clone();
            let api_url = api_url.clone();
            let model = model.clone();
            async move {
                match embed_faq(&db, &api_url, &model, faq_id).await {
                    Ok(_) => info!("Successfully embedded FAQ ID: {faq_id}"),
                    Err(e) => error!("Failed to embed FAQ ID: {faq_id}. Error: {e}"),
                }
            }
        })
        .await;

    let response = EmbedNewResponse {
        message: format!("Successfully processed {embed_count} FAQs."),
        embedded_articles: embed_count,
    };
    let debug_info =
        json!({ "limit": limit, "found": embed_count, "embedded_ids": faqs_to_embed_clone });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}
