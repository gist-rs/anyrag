//! # Local Embedding Service Integration Test
//!
//! This file contains a dedicated test to verify the functionality of the
//! external, OpenAI-compatible embedding service specified in the .env file.
//!
//! **Prerequisites:**
//! - A running local embedding server must be accessible at the URL defined
//!   in `EMBEDDINGS_API_URL` in your `.env` file.
//! - `dotenvy` is used to load the environment variables.

use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::env;

// --- Helper Structures for Deserialization ---

#[derive(Deserialize, Debug)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize, Debug)]
struct EmbeddingData {
    embedding: Vec<f32>,
    // We can ignore other fields like 'object' and 'index'
}

/// This test directly calls the configured embeddings API endpoint.
///
/// It verifies that the service is running and returns a response in the
/// expected format for a given input text.
#[tokio::test]
async fn test_local_embedding_service_responds_correctly() {
    // 1. Load environment variables from .env file
    dotenvy::dotenv().ok();
    let api_url = env::var("EMBEDDINGS_API_URL")
        .expect("EMBEDDINGS_API_URL must be set in your .env file for this test");
    let model = env::var("EMBEDDINGS_MODEL")
        .expect("EMBEDDINGS_MODEL must be set in your .env file for this test");

    println!("--- Testing Embedding Service ---");
    println!("API URL: {api_url}");
    println!("Model: {model}");

    // 2. Prepare the HTTP client and request body
    let client = Client::new();
    let request_body = json!({
        "input": "This is a test sentence.",
        "model": model,
    });

    // 3. Send the request to the embedding service
    let response_result = client.post(&api_url).json(&request_body).send().await;

    // --- Assertions ---

    // 3.1. Check if the request itself was successful
    assert!(
        response_result.is_ok(),
        "Request to embedding service failed. Is the service running at {api_url}?"
    );
    let response = response_result.unwrap();

    // 3.2. Check for a 200 OK HTTP status
    let status = response.status();
    let response_body_text = response
        .text()
        .await
        .unwrap_or_else(|_| "Could not read response body".to_string());
    assert_eq!(
        status, 200,
        "API did not return a 200 OK status. Response body: {response_body_text}"
    );

    // 3.3. Deserialize the JSON response
    let embedding_response: EmbeddingResponse = serde_json::from_str(&response_body_text)
        .expect("Failed to deserialize the JSON response from the embedding API.");

    // 3.4. Validate the structure and content of the response
    assert!(
        !embedding_response.data.is_empty(),
        "The 'data' array in the response should not be empty."
    );

    let embedding_data = &embedding_response.data[0];
    assert!(
        !embedding_data.embedding.is_empty(),
        "The 'embedding' vector should not be empty."
    );

    println!(
        "Successfully received an embedding vector with {} dimensions.",
        embedding_data.embedding.len()
    );

    // 3.5. A quick check that the vector contains floats
    assert!(embedding_data.embedding[0].is_finite());
}
