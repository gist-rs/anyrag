pub mod config;
mod errors;

use self::{
    config::{get_config, Config},
    errors::AppError,
};
use anyrag::embedding::{embed_and_update_article, search_articles_by_embedding, SearchResult};
use anyrag::ingest;
use anyrag::providers::ai::embedding::generate_embedding;
use anyrag::{
    providers::ai::{gemini::GeminiProvider, local::LocalAiProvider},
    ExecutePromptOptions, PromptClient, PromptClientBuilder,
};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::{debug, info};
use tracing_subscriber::FmtSubscriber;
use turso::{Builder, Database};

/// The shared application state.
///
/// This struct holds the `PromptClient` and default prompt templates
/// which are shared across all handlers.
#[derive(Clone)]
pub struct AppState {
    pub prompt_client: Arc<PromptClient>,
    pub db: Arc<Database>,
    pub embeddings_api_url: Option<String>,
    pub embeddings_model: Option<String>,
    pub query_system_prompt_template: Option<String>,
    pub query_user_prompt_template: Option<String>,
    pub format_system_prompt_template: Option<String>,
    pub format_user_prompt_template: Option<String>,
}

/// Builds the shared application state from the configuration.
///
/// This involves setting up the AI and storage providers.
pub async fn build_app_state(config: Config) -> anyhow::Result<AppState> {
    let ai_provider = match config.ai_provider.as_str() {
        "gemini" => {
            let api_key = config
                .ai_api_key
                .clone()
                .ok_or_else(|| anyhow::anyhow!("AI_API_KEY is required for the gemini provider"))?;
            Box::new(GeminiProvider::new(config.ai_api_url.clone(), api_key)?)
                as Box<dyn anyrag::providers::ai::AiProvider>
        }
        "local" => Box::new(LocalAiProvider::new(
            config.ai_api_url.clone(),
            config.ai_api_key.clone(),
            config.ai_model.clone(),
        )?) as Box<dyn anyrag::providers::ai::AiProvider>,
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported AI provider: {}",
                config.ai_provider
            ))
        }
    };

    let db = Builder::new_local(&config.db_url).build().await?;

    let prompt_client = PromptClientBuilder::new()
        .ai_provider(ai_provider)
        .bigquery_storage(config.project_id)
        .await?
        .build()?;

    Ok(AppState {
        prompt_client: Arc::new(prompt_client),
        db: Arc::new(db),
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
        .route("/search", post(search_handler))
        .with_state(app_state)
        .layer(TraceLayer::new_for_http())
}

/// The response body for the `/prompt` endpoint.
#[derive(Serialize)]
struct PromptResponse {
    result: String,
}

/// The root handler.
async fn root() -> &'static str {
    "anyrag server is running."
}

/// The health check handler.
async fn health_check() -> &'static str {
    "OK"
}

/// The handler for the `/prompt` endpoint.
///
/// This function takes a flexible JSON payload, combines it with server-side
/// default prompts (if any), and then executes it.
async fn prompt_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<PromptResponse>, AppError> {
    info!("Received prompt payload: '{}'", payload);

    let mut options: ExecutePromptOptions =
        serde_json::from_value(payload).map_err(anyrag::PromptError::from)?;

    // If the request doesn't specify a template, use the server's default from the environment.
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

/// The request body for the `/ingest` endpoint.
#[derive(Deserialize)]
struct IngestRequest {
    url: String,
}

/// The response body for the `/ingest` endpoint.
#[derive(Serialize)]
struct IngestResponse {
    message: String,
    ingested_articles: usize,
}

/// The handler for the `/ingest` endpoint.
///
/// This function fetches an RSS feed and saves the articles to the database.
async fn ingest_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<IngestRequest>,
) -> Result<Json<IngestResponse>, AppError> {
    info!("Received ingest request for URL: {}", payload.url);

    let ingested_count = ingest::ingest_from_url(&app_state.db, &payload.url).await?;

    let response = IngestResponse {
        message: "Ingestion successful".to_string(),
        ingested_articles: ingested_count,
    };

    Ok(Json(response))
}

/// The request body for the `/embed` endpoint.
#[derive(Deserialize)]
struct EmbedRequest {
    article_id: i64,
}

/// The request body for the `/search` endpoint.
#[derive(Deserialize)]
struct SearchRequest {
    query: String,
}

/// The handler for the `/embed` endpoint.
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

    embed_and_update_article(&app_state.db, api_url, model, payload.article_id).await?;

    Ok(Json(json!({ "success": true })))
}

/// The handler for the `/search` endpoint.
async fn search_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<Vec<SearchResult>>, AppError> {
    info!("Received search request for query: {}", payload.query);

    let api_url = app_state
        .embeddings_api_url
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("EMBEDDINGS_API_URL not set")))?;
    let model = app_state
        .embeddings_model
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("EMBEDDINGS_MODEL not set")))?;

    // 1. Generate an embedding for the search query.
    let query_vector = generate_embedding(api_url, model, &payload.query).await?;

    // 2. Search for similar articles in the database.
    // For now, let's use a hardcoded limit.
    let results = search_articles_by_embedding(&app_state.db, query_vector, 5).await?;

    Ok(Json(results))
}

/// The main entry point for running the server.
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

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Client;
    use serde_json::json;
    use tokio::net::TcpListener;
    use tokio::time::{sleep, Duration};

    async fn spawn_app() -> String {
        dotenvy::dotenv().ok();
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .compact()
            .try_init();

        // The test loads its own config but binds to a random port to avoid conflicts.
        let config = get_config().expect("Failed to read configuration for test");

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind random port");
        let port = listener.local_addr().unwrap().port();
        let address = format!("http://127.0.0.1:{port}");

        tokio::spawn(async move {
            if let Err(e) = run(listener, config).await {
                eprintln!("Server error: {e}");
            }
        });

        // Give the server a moment to start
        sleep(Duration::from_millis(100)).await;

        address
    }

    #[tokio::test]
    async fn test_e2e_prompt_execution() {
        let address = spawn_app().await;
        let client = Client::new();

        let payload = json!({
            "prompt": "What is the total word_count for the corpus 'kinghenryv'?",
            "table_name": "bigquery-public-data.samples.shakespeare",
            "instruction": "Answer with only the number, with thousand format."
        });

        let response = client
            .post(format!("{address}/prompt"))
            .json(&payload)
            .send()
            .await
            .expect("Failed to execute request.");

        assert!(
            response.status().is_success(),
            "Request failed with status: {}",
            response.status()
        );

        let body: serde_json::Value = response
            .json()
            .await
            .expect("Failed to parse response JSON");

        let result = body["result"]
            .as_str()
            .expect("Result field is not a string");

        println!("E2E Test Response from server: '{result}'");
        assert!(
            result.contains("27,894"),
            "Response did not contain the expected result."
        );
    }
}
