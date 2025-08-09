mod config;
mod errors;

use crate::{config::get_config, errors::AppError};
use anyrag::{
    providers::ai::{gemini::GeminiProvider, local::LocalAiProvider},
    ExecutePromptOptions, PromptClient, PromptClientBuilder,
};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::{debug, info};
use tracing_subscriber::FmtSubscriber;

/// The shared application state.
///
/// This struct holds the `PromptClient` and default prompt templates
/// which are shared across all handlers.
#[derive(Clone)]
struct AppState {
    prompt_client: Arc<PromptClient>,
    system_prompt_template: Option<String>,
    user_prompt_template: Option<String>,
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

    // If the request doesn't specify a system prompt, use the server's default.
    if options.system_prompt_template.is_none() {
        options.system_prompt_template = app_state.system_prompt_template.clone();
    }

    // If the request doesn't specify a user prompt, use the server's default.
    if options.user_prompt_template.is_none() {
        options.user_prompt_template = app_state.user_prompt_template.clone();
    }

    let result = app_state
        .prompt_client
        .execute_prompt_with_options(options)
        .await?;

    Ok(Json(PromptResponse { result }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Initialize tracing subscriber
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Get configuration
    let config = get_config()?;
    debug!(?config, "Server configuration loaded");

    // Build the AI provider based on configuration
    let ai_provider = match config.ai_provider.as_str() {
        "gemini" => {
            let api_key = config
                .ai_api_key
                .ok_or_else(|| anyhow::anyhow!("AI_API_KEY is required for the gemini provider"))?;
            Box::new(GeminiProvider::new(config.ai_api_url, api_key)?)
                as Box<dyn anyrag::providers::ai::AiProvider>
        }
        "local" => Box::new(LocalAiProvider::new(
            config.ai_api_url,
            config.ai_api_key,
            config.ai_model,
        )?) as Box<dyn anyrag::providers::ai::AiProvider>,
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported AI provider: {}",
                config.ai_provider
            ))
        }
    };

    // Build the PromptClient
    let prompt_client = PromptClientBuilder::new()
        .ai_provider(ai_provider)
        .bigquery_storage(config.project_id)
        .await?
        .build()?;

    // Create the application state
    let app_state = AppState {
        prompt_client: Arc::new(prompt_client),
        system_prompt_template: config.system_prompt_template,
        user_prompt_template: config.user_prompt_template,
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
