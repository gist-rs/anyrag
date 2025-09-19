use crate::{errors::PromptError, providers::ai::AiProvider};
use async_trait::async_trait;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fmt::Debug;
use tracing::{debug, info};

// --- OpenAI-compatible request and response structures ---

#[derive(Serialize, Debug)]
struct LocalAiRequest<'a> {
    messages: Vec<LocalAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<&'a str>,
    temperature: f32,
    max_tokens: i32,
    stream: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LocalAiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize, Debug)]
struct LocalAiResponse {
    choices: Vec<LocalAiChoice>,
}

#[derive(Deserialize, Debug)]
struct LocalAiChoice {
    message: LocalAiMessage,
}

// --- Local Provider implementation ---

/// A provider for interacting with a local or OpenAI-compatible API.
#[derive(Clone, Debug)]
pub struct LocalAiProvider {
    client: ReqwestClient,
    api_url: String,
    api_key: Option<String>,
    model: Option<String>,
}

impl LocalAiProvider {
    /// Creates a new `LocalAiProvider`.
    pub fn new(
        api_url: String,
        api_key: Option<String>,
        model: Option<String>,
    ) -> Result<Self, PromptError> {
        let client = ReqwestClient::builder()
            .build()
            .map_err(PromptError::ReqwestClientBuild)?;
        Ok(Self {
            client,
            api_url,
            api_key,
            model,
        })
    }
}

#[async_trait]
impl AiProvider for LocalAiProvider {
    /// Generates a response from a given system and user prompt.
    async fn generate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, PromptError> {
        let messages = vec![
            LocalAiMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            LocalAiMessage {
                role: "user".to_string(),
                content: user_prompt.to_string(),
            },
        ];

        let request_body = LocalAiRequest {
            messages,
            model: self.model.as_deref(),
            temperature: 0.0,
            max_tokens: 8192,
            stream: false,
        };

        debug!(payload = ?request_body, "--> Sending request to Local AI");
        println!(
            "-- AI Request Body: {}",
            serde_json::to_string_pretty(&request_body).unwrap()
        );
        debug!(payload = ?request_body, "--> Sending request to Local AI");
        info!("--> Local AI Provider using API URL: {}", self.api_url);
        let mut request_builder = self.client.post(&self.api_url);

        if let Some(key) = &self.api_key {
            request_builder = request_builder.bearer_auth(key);
        }

        let response = request_builder
            .json(&request_body)
            .send()
            .await
            .map_err(PromptError::AiRequest)?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(PromptError::AiApi(error_text));
        }

        let local_ai_response: LocalAiResponse = response
            .json()
            .await
            .map_err(PromptError::AiDeserialization)?;

        let raw_response = local_ai_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(raw_response)
    }
}
