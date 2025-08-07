mod config;
mod errors;

use crate::{config::get_config, errors::AppError};
use anyquery::{PromptClient, PromptClientBuilder};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// The shared application state.
///
/// This struct holds the `PromptClient` which is shared across all handlers.
#[derive(Clone)]
struct AppState {
    prompt_client: Arc<PromptClient>,
}

/// The request body for the `/prompt` endpoint.
#[derive(Deserialize)]
struct PromptRequest {
    prompt: String,
    table_name: Option<String>,
    instruction: Option<String>,
    answer_key: Option<String>,
}

/// The response body for the `/prompt` endpoint.
#[derive(Serialize)]
struct PromptResponse {
    result: String,
}

/// The root handler.
async fn root() -> &'static str {
    "BigQuery Tools Server is running."
}

/// The health check handler.
async fn health_check() -> &'static str {
    "OK"
}

/// The handler for the `/prompt` endpoint.
///
/// This function takes a prompt and an optional table name, uses the `PromptClient`
/// to execute it, and returns the result.
async fn prompt_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<PromptRequest>,
) -> Result<Json<PromptResponse>, AppError> {
    info!("Received prompt: '{}'", payload.prompt);

    let result = app_state
        .prompt_client
        .execute_prompt(
            &payload.prompt,
            payload.table_name.as_deref(),
            payload.instruction.as_deref(),
            payload.answer_key.as_deref(),
        )
        .await?;

    Ok(Json(PromptResponse { result }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Initialize tracing subscriber
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Get configuration
    let config = get_config()?;

    // Build the PromptClient
    let prompt_client = PromptClientBuilder::new()
        .gemini_url(config.gemini_api_url)
        .gemini_api_key(config.gemini_api_key)
        .bigquery_storage(config.project_id)
        .await?
        .build()?;

    // Create the application state
    let app_state = AppState {
        prompt_client: Arc::new(prompt_client),
    };

    // Build our application with routes
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/prompt", post(prompt_handler))
        .with_state(app_state)
        .layer(TraceLayer::new_for_http());

    // Run our app with hyper
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
