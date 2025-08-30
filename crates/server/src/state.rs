use anyrag::{
    providers::{
        ai::{gemini::GeminiProvider, local::LocalAiProvider},
        db::sqlite::SqliteProvider,
    },
    PromptClient, PromptClientBuilder,
};
use std::sync::Arc;

use super::config::Config;

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

    // Conditionally build the prompt client.
    let prompt_client = {
        let builder = PromptClientBuilder::new().ai_provider(ai_provider);

        #[cfg(feature = "bigquery")]
        if let Some(project_id) = config.project_id {
            tracing::info!(
                "BIGQUERY_PROJECT_ID found, building prompt client with BigQuery storage."
            );
            builder.bigquery_storage(project_id).await?.build()?
        } else {
            tracing::info!(
                "No BIGQUERY_PROJECT_ID found, building prompt client with SQLite storage."
            );
            builder
                .storage_provider(Box::new(sqlite_provider.clone()))
                .build()?
        }

        #[cfg(not(feature = "bigquery"))]
        {
            tracing::info!(
                "'bigquery' feature not enabled, building prompt client with SQLite storage."
            );
            builder
                .storage_provider(Box::new(sqlite_provider.clone()))
                .build()?
        }
    };

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
