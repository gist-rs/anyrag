use crate::{errors::PromptError, providers::ai::AiProvider};
use async_trait::async_trait;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::debug;

// --- Gemini-specific request and response structures ---

#[derive(Debug, Serialize)]
struct GenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: i32,
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Part {
    text: String,
}

#[derive(Deserialize, Debug)]
struct GeminiResponse {
    // It's possible for the API to return no candidates if the prompt is blocked.
    #[serde(default)]
    candidates: Vec<Candidate>,
    #[serde(rename = "promptFeedback")]
    prompt_feedback: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    content: ContentResponse,
    finish_reason: Option<String>,
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
    /// Generates a response from a given prompt.
    async fn generate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, PromptError> {
        let combined_prompt = format!("{system_prompt}\n\n{user_prompt}");
        let request_body = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: combined_prompt,
                }],
            }],
            generation_config: None,
        };

        debug!(payload = ?request_body, "--> Sending request to Gemini");

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

        if let Some(feedback) = &gemini_response.prompt_feedback {
            if !gemini_response.candidates.is_empty() {
                // Sometimes a non-fatal warning is returned, let's just log it.
                debug!("Gemini API returned prompt feedback: {:?}", feedback);
            } else {
                // If there are no candidates, it was a hard block.
                return Err(PromptError::AiApi(format!(
                    "Gemini API blocked the prompt due to safety settings. Feedback: {feedback}"
                )));
            }
        }

        if let Some(first_candidate) = gemini_response.candidates.first() {
            if let Some(reason) = &first_candidate.finish_reason {
                if reason != "STOP" {
                    return Err(PromptError::AiApi(format!(
                        "Gemini generation finished for a non-standard reason: {reason}. The response may be incomplete."
                    )));
                }
            }

            let raw_response = first_candidate
                .content
                .parts
                .first()
                .map(|p| p.text.clone())
                .unwrap_or_default();
            Ok(raw_response)
        } else {
            Err(PromptError::AiApi(
                "Gemini API returned no candidates and no feedback.".to_string(),
            ))
        }
    }
}
