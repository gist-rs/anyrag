//! ZAI provider implementation

use crate::errors::PromptError;
use crate::providers::ai::AiProvider;
use async_trait::async_trait;
use rig::client::CompletionClient;
use rig::completion::{CompletionModel as RigCompletionModel, Message};
use rig::OneOrMany;
use std::fmt::Debug;
use tracing::debug;

/// ZAI provider for completion-only operations
#[derive(Debug, Clone)]
pub struct ZaiProvider {
    pub client: crate::providers::ai::zai::Client,
    pub model: String,
}

impl ZaiProvider {
    /// Create a new ZAI provider
    pub fn new(
        client: crate::providers::ai::zai::Client,
        model: String,
    ) -> Result<Self, PromptError> {
        Ok(Self { client, model })
    }

    /// Create a ZAI provider from environment variables
    pub fn from_env() -> Result<Self, PromptError> {
        let api_key = std::env::var("AI_API_KEY").map_err(|_| {
            PromptError::MissingAiProvider(
                "AI_API_KEY environment variable not set for ZAI provider".to_string(),
            )
        })?;

        let client = super::Client::builder(&api_key).build();

        Ok(Self {
            client,
            model: "glm-4.6".to_string(),
        })
    }
}

#[async_trait]
impl AiProvider for ZaiProvider {
    async fn generate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, PromptError> {
        debug!(
            "Generating response with ZAI provider using model: {}",
            self.model
        );

        // Create a completion request
        let mut chat_history = Vec::new();

        // Add system message if provided
        if !system_prompt.is_empty() {
            chat_history.push(Message::user(format!("System: {system_prompt}")));
        }

        // Add user message
        chat_history.push(Message::user(user_prompt));

        // Create completion request
        let completion_request = rig::completion::CompletionRequest {
            preamble: if system_prompt.is_empty() {
                None
            } else {
                Some(system_prompt.to_string())
            },
            chat_history: OneOrMany::many(chat_history).expect("Chat history should not be empty"),
            documents: vec![],
            temperature: None,
            max_tokens: None,
            tools: vec![],
            additional_params: None,
        };

        // Send the completion request
        let completion_model = self.client.completion_model(&self.model);
        let response = completion_model
            .completion(completion_request)
            .await
            .map_err(|e| PromptError::AiApi(format!("ZAI completion error: {e}")))?;

        // Extract the content from the response
        let content = response
            .raw_response
            .choices
            .first()
            .and_then(|choice| choice.message.as_ref())
            .and_then(|msg| match msg {
                crate::providers::ai::zai::completion::ZaiMessage::Assistant {
                    content, ..
                } => content.as_ref(),
                _ => None,
            })
            .ok_or_else(|| PromptError::AiApi("Empty response from ZAI".to_string()))?;

        Ok(content.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::ai::zai::Client;
    use anyhow::Result;

    #[tokio::test]
    async fn test_zai_provider_from_env() -> Result<()> {
        // Test with a mock environment
        std::env::set_var("AI_API_KEY", "test-key");

        let provider = ZaiProvider::from_env();

        // This will fail during actual API call, but confirms creation works
        assert!(provider.is_ok());

        std::env::remove_var("AI_API_KEY");
        Ok(())
    }

    #[tokio::test]
    async fn test_zai_provider_direct_creation() -> Result<()> {
        let client = Client::builder("test-key").build();
        let provider = ZaiProvider::new(client, "glm-4.6".to_string())?;

        assert_eq!(provider.model, "glm-4.6");
        Ok(())
    }
}
