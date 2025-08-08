use crate::{errors::PromptError, providers::ai::AiProvider};
use async_trait::async_trait;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

// --- Gemini-specific request and response structures ---

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Deserialize, Debug)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize, Debug)]
struct Candidate {
    content: ContentResponse,
}

#[derive(Deserialize, Debug)]
struct ContentResponse {
    parts: Vec<PartResponse>,
}

#[derive(Deserialize, Debug)]
struct PartResponse {
    text: String,
}

// --- Gemini Provider implementation ---

/// A provider for interacting with the Google Gemini API.
#[derive(Clone, Debug)]
pub struct GeminiProvider {
    client: ReqwestClient,
    api_url: String,
    api_key: String,
}

impl GeminiProvider {
    /// Creates a new `GeminiProvider`.
    pub fn new(api_url: String, api_key: String) -> Result<Self, PromptError> {
        let client = ReqwestClient::builder()
            .build()
            .map_err(PromptError::ReqwestClientBuild)?;
        Ok(Self {
            client,
            api_url,
            api_key,
        })
    }
}

#[async_trait]
impl AiProvider for GeminiProvider {
    /// Generates a SQL query using the Gemini API.
    async fn generate_sql(&self, prompt: &str) -> Result<String, PromptError> {
        let request_body = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
            }],
        };

        let response = self
            .client
            .post(&self.api_url)
            .query(&[("key", &self.api_key)])
            .json(&request_body)
            .send()
            .await
            .map_err(PromptError::AiRequest)?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(PromptError::AiApi(error_text));
        }

        let gemini_response: GeminiResponse = response
            .json()
            .await
            .map_err(PromptError::AiDeserialization)?;

        let raw_response = gemini_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .unwrap_or_default();

        Ok(raw_response)
    }
}
