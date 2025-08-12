use super::{errors::AppError, state::AppState};
use anyrag::{
    ingest::{
        embed_article, ingest_from_google_sheet_url, ingest_from_url,
        sheet_url_to_export_url_and_table_name,
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
    result: String,
}

#[derive(Deserialize)]
pub struct IngestRequest {
    url: String,
}

#[derive(Serialize)]
pub struct IngestResponse {
    message: String,
    ingested_articles: usize,
}

#[derive(Deserialize)]
pub struct EmbedRequest {
    article_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct EmbedNewRequest {
    limit: Option<usize>,
}

#[derive(Serialize, Debug)]
pub struct EmbedNewResponse {
    message: String,
    embedded_articles: usize,
}

#[derive(Deserialize)]
pub struct SearchRequest {
    query: String,
    limit: Option<u32>,
    #[serde(default)] // This makes the field optional, defaulting to SearchMode::default()
    mode: SearchMode,
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

    // Check for a Google Sheet URL in the prompt.
    let sheet_url = options
        .prompt
        .split_whitespace()
        .find(|word| word.contains("/spreadsheets/d/"));

    if let Some(url) = sheet_url {
        info!("Detected Google Sheet URL in prompt: {}", url);

        // 1. Get the potential table name and export URL from the sheet URL.
        let (export_url, table_name) = sheet_url_to_export_url_and_table_name(url)
            .map_err(|e| anyhow::anyhow!("Sheet URL transformation failed: {e}"))?;

        // 2. Check if the table already exists using a robust schema check.
        match app_state
            .sqlite_provider
            .get_table_schema(&table_name)
            .await
        {
            Ok(_) => {
                info!("Table '{table_name}' already exists, skipping ingestion.");
            }
            // If it's a "not found" error, we proceed to ingest.
            Err(anyrag::PromptError::StorageOperationFailed(e)) if e.contains("not found") => {
                info!("Table '{table_name}' not found, proceeding with ingestion.");
                ingest_from_google_sheet_url(
                    &app_state.sqlite_provider.db,
                    &export_url,
                    &table_name,
                )
                .await
                .map_err(|e| anyhow::anyhow!("Sheet ingestion failed: {e}"))?;
            }
            // For any other schema error, we should fail fast.
            Err(e) => return Err(e.into()),
        };

        // 3. Update the prompt options to target the now-guaranteed-to-exist table.
        options.table_name = Some(table_name);

        // 4. For this request, create a special client that's configured to talk to SQLite
        //    instead of the default BigQuery provider.
        let sqlite_prompt_client = PromptClientBuilder::new()
            .ai_provider(app_state.prompt_client.ai_provider.clone())
            .storage_provider(Box::new(app_state.sqlite_provider.as_ref().clone()))
            .build()?;

        // Apply server-wide default prompts if not provided in the request.
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

        // Execute the prompt using the temporary SQLite client.
        let result = sqlite_prompt_client
            .execute_prompt_with_options(options)
            .await?;

        return Ok(Json(PromptResponse { result }));
    }

    // --- Default Flow (if no sheet URL is detected) ---

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

    let result = app_state
        .prompt_client
        .execute_prompt_with_options(options)
        .await?;

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
