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
    input: &'a [&'a str],
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
struct GeminiBatchEmbeddingRequest<'a> {
    requests: Vec<GeminiEmbeddingRequest<'a>>,
}

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
struct GeminiBatchEmbeddingResponse {
    embeddings: Vec<GeminiEmbeddingValue>,
}

#[derive(Deserialize, Debug)]
struct GeminiEmbeddingValue {
    values: Vec<f32>,
}

/// Generates vector embeddings for a given batch of text inputs using an external API.
///
/// This function dynamically constructs the correct JSON payload based on whether
/// the `api_url` is for a Gemini or an OpenAI-compatible endpoint.
pub async fn generate_embeddings_batch(
    api_url: &str,
    model: &str,
    inputs: &[&str],
    api_key: Option<&str>,
) -> Result<Vec<Vec<f32>>, PromptError> {
    if inputs.is_empty() {
        return Ok(Vec::new());
    }

    let client = ReqwestClient::new();
    // The Gemini batch endpoint is different.
    let final_api_url = if api_url.ends_with(":embedContent") {
        api_url.replace(":embedContent", ":batchEmbedContents")
    } else {
        api_url.to_string()
    };
    let mut request_builder = client.post(&final_api_url);
    let is_gemini = final_api_url.contains("generativelanguage.googleapis.com");

    // --- 1. Construct the appropriate request body and apply auth ---
    if is_gemini {
        let gemini_model_name = if model.starts_with("models/") {
            model.to_string()
        } else {
            format!("models/{model}")
        };

        let requests = inputs
            .iter()
            .map(|&text| GeminiEmbeddingRequest {
                model: gemini_model_name.clone(),
                content: GeminiEmbeddingContent {
                    parts: vec![GeminiEmbeddingPart { text }],
                },
            })
            .collect();

        let request_body = GeminiBatchEmbeddingRequest { requests };
        debug!(payload = ?request_body, "--> Sending BATCH request to Gemini Embeddings API");
        request_builder = request_builder.json(&request_body);
        if let Some(key) = api_key {
            request_builder = request_builder.header("x-goog-api-key", key);
        }
    } else {
        let request_body = OpenAIEmbeddingRequest {
            model,
            input: inputs,
        };
        debug!(payload = ?request_body, "--> Sending BATCH request to OpenAI-compatible Embeddings API");
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

    let status = response.status();
    let response_text = response.text().await.unwrap_or_default();

    if !status.is_success() {
        debug!(response_body = %response_text, "<- Received non-success response from embeddings API");
        return Err(PromptError::AiApi(response_text));
    }

    debug!(response_body = %response_text, "<- Received success response from embeddings API");

    if is_gemini {
        let gemini_response: GeminiBatchEmbeddingResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                PromptError::AiApi(format!("Deserialization error: {e}. Body: {response_text}"))
            })?;
        Ok(gemini_response
            .embeddings
            .into_iter()
            .map(|e| e.values)
            .collect())
    } else {
        let openai_response: OpenAIEmbeddingResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                PromptError::AiApi(format!("Deserialization error: {e}. Body: {response_text}"))
            })?;

        Ok(openai_response
            .data
            .into_iter()
            .map(|d| d.embedding)
            .collect())
    }
}
