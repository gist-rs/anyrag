use super::{errors::AppError, state::AppState};
use anyrag::types::ContentType;
use anyrag::{
    ingest::{
        embed_article, embed_faq, export_for_finetuning, ingest_from_google_sheet_url,
        ingest_from_url, run_ingestion_pipeline, sheet_url_to_export_url_and_table_name,
    },
    providers::{
        ai::generate_embedding,
        db::storage::{KeywordSearch, Storage, VectorSearch},
    },
    search::{hybrid_search, SearchMode},
    ExecutePromptOptions, PromptClientBuilder, SearchResult,
};
use axum::{extract::State, Json};
use futures::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{error, info};
use turso::Value as TursoValue;

// --- API Payloads ---

#[derive(Serialize)]
pub struct PromptResponse {
    pub result: String,
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
    #[serde(default)] // This makes the field optional, defaulting to SearchMode::default()
    pub mode: SearchMode,
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
    Json(payload): Json<Value>,
) -> Result<Json<PromptResponse>, AppError> {
    info!("Received prompt payload: '{}'", payload);

    let mut options: ExecutePromptOptions =
        serde_json::from_value(payload).map_err(anyrag::PromptError::from)?;

    // Apply server-wide default prompts first. If these are not set, the library's
    // own defaults will be used. This allows server admins to customize behavior.
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

    // Check for a Google Sheet URL in the prompt to trigger the special ingestion flow.
    let sheet_url = options
        .prompt
        .split_whitespace()
        .find(|word| word.contains("/spreadsheets/d/"));

    let result = if let Some(url) = sheet_url {
        // --- Google Sheet Flow ---
        info!("Detected Google Sheet URL in prompt: {}", url);

        let (export_url, table_name) = sheet_url_to_export_url_and_table_name(url)
            .map_err(|e| anyhow::anyhow!("Sheet URL transformation failed: {e}"))?;

        // Ingest the sheet data if the corresponding table doesn't already exist.
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

        // Create a temporary client that uses the SQLite provider for this request.
        let sqlite_prompt_client = PromptClientBuilder::new()
            .ai_provider(app_state.prompt_client.ai_provider.clone())
            .storage_provider(Box::new(app_state.sqlite_provider.as_ref().clone()))
            .build()?;

        // Execute the prompt. The library will now automatically select the correct
        // SQL dialect prompts because the storage provider is SQLite.
        sqlite_prompt_client
            .execute_prompt_with_options(options)
            .await?
    } else {
        // --- Default Flow (BigQuery) ---
        app_state
            .prompt_client
            .execute_prompt_with_options(options)
            .await?
    };

    Ok(Json(PromptResponse { result }))
}

pub async fn ingest_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<IngestRequest>,
) -> Result<Json<IngestResponse>, AppError> {
    info!("Received ingest request for URL: {}", payload.url);
    let ingested_count = ingest_from_url(&app_state.sqlite_provider.db, &payload.url).await?;
    let response = IngestResponse {
        message: "Ingestion successful".to_string(),
        ingested_articles: ingested_count,
    };
    Ok(Json(response))
}

pub async fn embed_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<EmbedRequest>,
) -> Result<Json<Value>, AppError> {
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

    Ok(Json(json!({ "success": true })))
}

pub async fn embed_new_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<EmbedNewRequest>,
) -> Result<Json<EmbedNewResponse>, AppError> {
    let limit = payload.limit.unwrap_or(10);
    info!("Received request to embed up to {limit} new articles.");

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
        "SELECT id FROM articles WHERE embedding IS NULL ORDER BY pub_date DESC LIMIT {limit}"
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
        return Ok(Json(EmbedNewResponse {
            message: "No new articles to embed.".to_string(),
            embedded_articles: 0,
        }));
    }

    stream::iter(articles_to_embed)
        .for_each_concurrent(1, |article_id| {
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

    Ok(Json(EmbedNewResponse {
        message: format!("Successfully processed {embed_count} articles."),
        embedded_articles: embed_count,
    }))
}

pub async fn vector_search_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<Vec<SearchResult>>, AppError> {
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
        .vector_search(query_vector, limit)
        .await?;

    Ok(Json(results))
}

pub async fn keyword_search_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<Vec<SearchResult>>, AppError> {
    let limit = payload.limit.unwrap_or(10);
    info!(
        "Received keyword search request for query: '{}', limit: {}",
        payload.query, limit
    );
    let results = app_state
        .sqlite_provider
        .keyword_search(&payload.query, limit)
        .await?;
    Ok(Json(results))
}

pub async fn hybrid_search_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<Vec<SearchResult>>, AppError> {
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
        limit,
        payload.mode,
    )
    .await?;

    Ok(Json(results))
}

pub async fn knowledge_ingest_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<IngestRequest>,
) -> Result<Json<KnowledgeIngestResponse>, AppError> {
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
    Ok(Json(response))
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
    Json(payload): Json<SearchRequest>,
) -> Result<Json<PromptResponse>, AppError> {
    let limit = payload.limit.unwrap_or(5); // Use a smaller limit for context
    info!(
        "Received knowledge RAG search for query: '{}', limit: {}",
        payload.query, limit
    );

    // 1. Get embedding provider details from app state
    let api_url = app_state
        .embeddings_api_url
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("EMBEDDINGS_API_URL not set")))?;
    let model = app_state
        .embeddings_model
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("EMBEDDINGS_MODEL not set")))?;

    // 2. Generate embedding for the search query
    let query_vector = generate_embedding(api_url, model, &payload.query).await?;

    // 3. Perform a vector search to find relevant knowledge base entries.
    let search_results = app_state
        .sqlite_provider
        .vector_search_faqs(query_vector, limit)
        .await?;

    // 4. Build context from search results and handle empty case
    if search_results.is_empty() {
        return Ok(Json(PromptResponse {
            result: "I could not find any relevant information in the knowledge base to answer your question.".to_string(),
        }));
    }

    let context = search_results
        .iter()
        .map(|result| format!("- {}", result.answer))
        .collect::<Vec<String>>()
        .join("\n\n");

    // 5. Use the prompt client to generate a synthesized answer
    let options = ExecutePromptOptions {
        prompt: payload.query,
        content_type: Some(ContentType::Knowledge),
        context: Some(context),
        instruction: payload.instruction,
        ..Default::default()
    };

    let result = app_state
        .prompt_client
        .execute_prompt_with_options(options)
        .await?;

    Ok(Json(PromptResponse { result }))
}

pub async fn embed_faqs_new_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<EmbedNewRequest>,
) -> Result<Json<EmbedNewResponse>, AppError> {
    let limit = payload.limit.unwrap_or(20); // Default to a higher limit for batch jobs
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
        return Ok(Json(EmbedNewResponse {
            message: "No new FAQs to embed.".to_string(),
            embedded_articles: 0,
        }));
    }

    stream::iter(faqs_to_embed)
        .for_each_concurrent(1, |faq_id| {
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

    Ok(Json(EmbedNewResponse {
        message: format!("Successfully processed {embed_count} FAQs."),
        embedded_articles: embed_count,
    }))
}
