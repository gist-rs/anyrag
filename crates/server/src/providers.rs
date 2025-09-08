//! # Dynamic AI Provider Factory
//!
//! This module centralizes the logic for creating AI provider instances dynamically
//! at request time. This is used when a user's request overrides the default model
//! configured for a task.

use crate::{errors::AppError, state::AppState};
use anyrag::providers::ai::{gemini::GeminiProvider, local::LocalAiProvider, AiProvider};
use tracing::{info, warn};

/// A tuple containing the instantiated provider and the name of the model it's configured for.
pub type DynamicProviderResult = (Box<dyn AiProvider>, String);

/// Creates an AI provider instance dynamically based on a model name specified in a request.
///
/// This function handles the logic for:
/// - Differentiating between Gemini and local models.
/// - Sourcing API keys and URLs from the environment and configuration.
/// - **Providing a robust fallback for the local AI provider's URL** if it's not
///   explicitly configured, preventing "builder errors".
pub async fn create_dynamic_provider(
    app_state: &AppState,
    model_name: &str,
) -> Result<DynamicProviderResult, AppError> {
    info!(
        "Request to create dynamic provider for model: '{}'",
        model_name
    );

    let provider: Box<dyn AiProvider> = if model_name.starts_with("gemini") {
        let api_key = std::env::var("AI_API_KEY").map_err(|_| {
            AppError::Internal(anyhow::anyhow!(
                "AI_API_KEY must be set in .env to use Gemini models dynamically."
            ))
        })?;
        let api_url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{model_name}:generateContent"
        );
        info!(
            "Dynamically configuring Gemini provider with URL: {}",
            api_url
        );
        Box::new(GeminiProvider::new(api_url, api_key)?)
    } else {
        // --- Local Provider Logic with Fallback ---
        let local_provider_config = app_state.config.providers.get("local_default").ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("A 'local_default' provider must be defined in config.yml for local model overrides."))
        })?;

        let api_url = if local_provider_config.api_url.is_empty() {
            let fallback_url = "http://localhost:1234/v1/chat/completions";
            warn!(
                "LOCAL_AI_API_URL is not set in .env. Falling back to default: {}",
                fallback_url
            );
            fallback_url.to_string()
        } else {
            local_provider_config.api_url.clone()
        };

        info!(
            "Dynamically configuring Local AI provider with URL: {}",
            api_url
        );
        Box::new(LocalAiProvider::new(
            api_url,
            local_provider_config.api_key.clone(),
            Some(model_name.to_string()),
        )?)
    };

    Ok((provider, model_name.to_string()))
}
