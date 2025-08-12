pub mod config;
mod errors;

use self::{
    config::{get_config, Config},
    errors::AppError,
};
use anyrag::{
    ingest::{embed_article, ingest_from_google_sheet_url, ingest_from_url},
    providers::{
        ai::{gemini::GeminiProvider, generate_embedding, local::LocalAiProvider},
        db::{
            sqlite::SqliteProvider,
            storage::{KeywordSearch, VectorSearch},
        },
    },
    search::{hybrid_search, SearchMode},
    ExecutePromptOptions, PromptClient, PromptClientBuilder, SearchResult,
};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use futures::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::{debug, error, info};
use tracing_subscriber::FmtSubscriber;
use turso::Value as TursoValue;

/// The shared application state.
#[derive(Clone)]
pub struct AppState {
    pub prompt_client: Arc<PromptClient>,
    pub sqlite_provider: Arc<SqliteProvider>,
    pub embeddings_api_url: Option<String>,
    pub embeddings_model: Option<String>,
    pub query_system_prompt_template: Option<String>,
    pub query_user_prompt_template: Option<String>,
    pub format_system_prompt_template: Option<String>,
    pub format_user_prompt_template: Option<String>,
}

/// Builds the shared application state from the configuration.
pub async fn build_app_state(config: Config) -> anyhow::Result<AppState> {
    let ai_provider: Box<dyn anyrag::providers::ai::AiProvider> =
        match config.ai_provider.as_str() {
            "gemini" => {
                let api_key = config.ai_api_key.clone().ok_or_else(|| {
                    anyhow::anyhow!("AI_API_KEY is required for the gemini provider")
                })?;
                Box::new(GeminiProvider::new(config.ai_api_url.clone(), api_key)?)
            }
            "local" => Box::new(LocalAiProvider::new(
                config.ai_api_url.clone(),
                config.ai_api_key.clone(),
                config.ai_model.clone(),
            )?),
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported AI provider: {}",
                    config.ai_provider
                ))
            }
        };

    // The provider for local ingestion, embedding, and searching.
    let sqlite_provider = SqliteProvider::new(&config.db_url).await?;

    // The main prompt client for NL-to-SQL is configured for BigQuery.
    let prompt_client = PromptClientBuilder::new()
        .ai_provider(ai_provider)
        .bigquery_storage(config.project_id)
        .await?
        .build()?;

    Ok(AppState {
        prompt_client: Arc::new(prompt_client),
        sqlite_provider: Arc::new(sqlite_provider),
        embeddings_api_url: config.embeddings_api_url,
        embeddings_model: config.embeddings_model,
        query_system_prompt_template: config.query_system_prompt_template,
        query_user_prompt_template: config.query_user_prompt_template,
        format_system_prompt_template: config.format_system_prompt_template,
        format_user_prompt_template: config.format_user_prompt_template,
    })
}

/// Creates the Axum router with all the application routes.
pub fn create_router(app_state: AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/prompt", post(prompt_handler))
        .route("/ingest", post(ingest_handler))
        .route("/embed", post(embed_handler))
        .route("/embed/new", post(embed_new_handler))
        .route("/search/vector", post(vector_search_handler))
        .route("/search/keyword", post(keyword_search_handler))
        .route("/search/hybrid", post(hybrid_search_handler))
        .with_state(app_state)
        .layer(TraceLayer::new_for_http())
}

// --- API Payloads ---

#[derive(Serialize)]
struct PromptResponse {
    result: String,
}

#[derive(Deserialize)]
struct IngestRequest {
    url: String,
}

#[derive(Serialize)]
struct IngestResponse {
    message: String,
    ingested_articles: usize,
}

#[derive(Deserialize)]
struct EmbedRequest {
    article_id: i64,
}

#[derive(Deserialize, Debug)]
struct EmbedNewRequest {
    limit: Option<usize>,
}

#[derive(Serialize, Debug)]
struct EmbedNewResponse {
    message: String,
    embedded_articles: usize,
}

#[derive(Deserialize)]
struct SearchRequest {
    query: String,
    limit: Option<u32>,
    #[serde(default)] // This makes the field optional, defaulting to SearchMode::default()
    mode: SearchMode,
}

// --- Route Handlers ---

async fn root() -> &'static str {
    "anyrag server is running."
}

async fn health_check() -> &'static str {
    "OK"
}

async fn prompt_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<PromptResponse>, AppError> {
    info!("Received prompt payload: '{}'", payload);

    let mut options: ExecutePromptOptions =
        serde_json::from_value(payload).map_err(anyrag::PromptError::from)?;

    // Check for a Google Sheet URL in the prompt.
    // This is a simple but effective way to detect the URL without adding a regex dependency.
    let sheet_url = options
        .prompt
        .split_whitespace()
        .find(|word| word.contains("/spreadsheets/d/"));

    if let Some(url) = sheet_url {
        info!("Detected Google Sheet URL in prompt: {}", url);

        // Ingest the sheet into a new table in the local SQLite database.
        let (table_name, _rows_ingested) =
            ingest_from_google_sheet_url(&app_state.sqlite_provider.db, url)
                .await
                .map_err(|e| anyhow::anyhow!("Sheet ingestion failed: {e}"))?;

        // Update the prompt options to target the newly created table.
        options.table_name = Some(table_name);

        // For this request, we need a special client that's configured to talk to SQLite
        // instead of the default BigQuery provider.
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

async fn ingest_handler(
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

async fn embed_handler(
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

async fn embed_new_handler(
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

async fn vector_search_handler(
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

async fn keyword_search_handler(
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

async fn hybrid_search_handler(
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

// --- Main Application ---

pub async fn run(listener: tokio::net::TcpListener, config: Config) -> anyhow::Result<()> {
    debug!(?config, "Server configuration loaded");

    let app_state = build_app_state(config).await?;
    let app = create_router(app_state);

    info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

#[tokio::main]
#[cfg_attr(test, allow(dead_code))]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let config = get_config()?;
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Server listening on {}", addr);
    run(listener, config).await
}
