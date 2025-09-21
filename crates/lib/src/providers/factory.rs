//! # Dynamic AI Provider Factory
//!
//! This module centralizes the logic for creating AI provider instances dynamically.
//! This is used when a user's request overrides the default model configured for a task.
//! By placing this logic in the `lib` crate, we allow any consumer (server, cli, etc.)
//! to leverage the same dynamic provider creation mechanism, ensuring consistency.

use crate::{
    errors::PromptError,
    providers::ai::{gemini::GeminiProvider, local::LocalAiProvider, AiProvider},
    types::ProviderConfig,
};
use std::collections::HashMap;
use tracing::info;

/// A tuple containing the instantiated provider and the name of the model it's configured for.
pub type DynamicProviderResult = (Box<dyn AiProvider>, String);

/// Creates an AI provider instance dynamically based on a model name specified in a request.
///
/// This function handles the logic for:
/// - Differentiating between Gemini and local models based on the model name.
/// - Sourcing API keys and URLs from the environment and the application configuration.
/// - Ensuring that the local AI provider's URL is configured to prevent runtime errors.
pub fn create_dynamic_provider(
    providers_config: &HashMap<String, ProviderConfig>,
    model_name: &str,
) -> Result<DynamicProviderResult, PromptError> {
    info!(
        "Request to create dynamic provider for model: '{}'",
        model_name
    );

    let provider: Box<dyn AiProvider> = if model_name.starts_with("gemini") {
        let api_key = std::env::var("AI_API_KEY").map_err(|_| {
            PromptError::MissingAiProvider(
                "AI_API_KEY must be set in .env to use Gemini models dynamically.".to_string(),
            )
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
        let local_provider_config = providers_config.get("local_default").ok_or_else(|| {
            PromptError::MissingAiProvider(
                "A 'local_default' provider must be defined in config.yml for local model overrides."
                    .to_string(),
            )
        })?;

        let api_url = local_provider_config.api_url.as_ref().cloned().ok_or_else(
            || {
                PromptError::MissingAiProvider(
                "api_url is not set for local_default provider in config.yml. Please set LOCAL_AI_API_URL in your .env file."
                    .to_string(),
            )
            },
        )?;

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
