//! # Embeddings Provider
//!
//! This module provides functionality for generating vector embeddings by calling
//! an external, OpenAI-compatible embeddings API.

use crate::errors::PromptError;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use tracing::debug;

// --- OpenAI-compatible request and response structures ---

#[derive(Serialize, Debug)]
struct OpenAIEmbeddingRequest<'a> {
    model: &'a str,
    input: &'a str,
}

#[derive(Deserialize, Debug)]
struct OpenAIEmbeddingResponse {
    data: Vec<OpenAIEmbeddingData>,
}

#[derive(Deserialize, Debug)]
struct OpenAIEmbeddingData {
    embedding: Vec<f32>,
}

// --- Gemini-specific request and response structures ---

#[derive(Serialize, Debug)]
struct GeminiEmbeddingRequest<'a> {
    model: String,
    content: GeminiEmbeddingContent<'a>,
}

#[derive(Serialize, Debug)]
struct GeminiEmbeddingContent<'a> {
    parts: Vec<GeminiEmbeddingPart<'a>>,
}

#[derive(Serialize, Debug)]
struct GeminiEmbeddingPart<'a> {
    text: &'a str,
}

#[derive(Deserialize, Debug)]
struct GeminiEmbeddingResponse {
    embedding: GeminiEmbeddingValue,
}

#[derive(Deserialize, Debug)]
struct GeminiEmbeddingValue {
    values: Vec<f32>,
}

/// Generates a vector embedding for a given text input using an external API.
///
/// This function dynamically constructs the correct JSON payload based on whether
/// the `api_url` is for a Gemini or an OpenAI-compatible endpoint.
pub async fn generate_embedding(
    api_url: &str,
    model: &str,
    input: &str,
    api_key: Option<&str>,
) -> Result<Vec<f32>, PromptError> {
    // info!("Generating embedding using model '{model}' with API URL: {api_url}");
    // info!(text_to_embed = %input, "Sending text for embedding");
    let client = ReqwestClient::new();
    let mut request_builder = client.post(api_url);
    let is_gemini = api_url.contains("generativelanguage.googleapis.com");

    // --- 1. Construct the appropriate request body and apply auth ---
    if is_gemini {
        // Gemini requires the model name to be prefixed with "models/" in the payload.
        let gemini_model_name = if model.starts_with("models/") {
            model.to_string()
        } else {
            format!("models/{model}")
        };

        let request_body = GeminiEmbeddingRequest {
            model: gemini_model_name,
            content: GeminiEmbeddingContent {
                parts: vec![GeminiEmbeddingPart { text: input }],
            },
        };
        debug!(payload = ?request_body, "--> Sending request to Gemini Embeddings API");
        request_builder = request_builder.json(&request_body);
        if let Some(key) = api_key {
            // Gemini uses an `x-goog-api-key` header for embeddings, not a query param.
            request_builder = request_builder.header("x-goog-api-key", key);
        }
    } else {
        let request_body = OpenAIEmbeddingRequest { model, input };
        debug!(payload = ?request_body, "--> Sending request to OpenAI-compatible Embeddings API");
        request_builder = request_builder.json(&request_body);
        if let Some(key) = api_key {
            request_builder = request_builder.bearer_auth(key);
        }
    }

    // --- 2. Send the request and handle the response ---
    let response = request_builder
        .send()
        .await
        .map_err(PromptError::AiRequest)?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(PromptError::AiApi(error_text));
    }

    if is_gemini {
        let gemini_response: GeminiEmbeddingResponse = response
            .json()
            .await
            .map_err(PromptError::AiDeserialization)?;
        Ok(gemini_response.embedding.values)
    } else {
        let openai_response: OpenAIEmbeddingResponse = response
            .json()
            .await
            .map_err(PromptError::AiDeserialization)?;

        openai_response
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| {
                PromptError::AiApi("OpenAI-compatible API returned no embeddings".to_string())
            })
    }
}
