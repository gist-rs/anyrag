//! # Embeddings Provider
//!
//! This module provides functionality for generating vector embeddings by calling
//! an external, OpenAI-compatible embeddings API.

use crate::errors::PromptError;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

// --- OpenAI-compatible request and response structures for embeddings ---

#[derive(Serialize, Debug)]
struct EmbeddingRequest<'a> {
    model: &'a str,
    input: &'a str,
}

#[derive(Deserialize, Debug)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize, Debug)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

/// Generates a vector embedding for a given text input using an external API.
///
/// # Arguments
///
/// * `api_url`: The full URL of the embeddings endpoint.
/// * `model`: The name of the embedding model to use.
/// * `input`: The text to be embedded.
///
/// # Returns
///
/// A `Result` containing the vector (`Vec<f32>`) on success, or a `PromptError` on failure.
pub async fn generate_embedding(
    api_url: &str,
    model: &str,
    input: &str,
) -> Result<Vec<f32>, PromptError> {
    info!("Generating embedding using model '{model}' with API URL: {api_url}");
    let client = ReqwestClient::new();
    let request_body = EmbeddingRequest { model, input };

    debug!(payload = ?serde_json::to_string(&request_body), "--> Sending request to Embeddings API");

    let response = client
        .post(api_url)
        .json(&request_body)
        .send()
        .await
        .map_err(PromptError::AiRequest)?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(PromptError::AiApi(error_text));
    }

    let embedding_response: EmbeddingResponse = response
        .json()
        .await
        .map_err(PromptError::AiDeserialization)?;

    // The API returns a list of embeddings, one for each input string.
    // Since we only send one string, we expect exactly one embedding back.
    embedding_response
        .data
        .into_iter()
        .next()
        .map(|d| d.embedding)
        .ok_or_else(|| PromptError::AiApi("Embeddings API returned no embeddings".to_string()))
}
