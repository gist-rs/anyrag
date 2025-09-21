//! # Embedding Service Test
//!
//! This file tests that the application correctly communicates with a configured
//! embedding service.

mod common;

use anyhow::Result;
use common::{generate_jwt, TestApp};
use httpmock::Method;
use serde_json::json;

/// This test verifies that an endpoint using the embedding service correctly
/// calls the mocked service endpoint.
#[tokio::test]
async fn test_app_calls_embedding_service() -> Result<()> {
    // 1. Arrange
    let app = TestApp::spawn("test_app_calls_embedding_service").await?;
    let mock_vector = vec![0.1, 0.2, 0.3];

    // 2. Mock the embedding service endpoint.
    // This is the service our app will call.
    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/test_app_calls_embedding_service/v1/embeddings")
            .body_contains("test query");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": mock_vector }] }));
    });

    // 3. Act: Call an endpoint on our app that triggers the embedding service.
    // The `/search/vector` endpoint is suitable for this. The database is empty,
    // so it will return no results, but it will still call the embedding service first.
    let token = generate_jwt("test-user@example.com")?;
    let response = app
        .client
        .post(format!("{}/search/vector", app.address))
        .bearer_auth(token)
        .json(&json!({ "query": "test query" }))
        .send()
        .await?;

    // 4. Assert
    // Check that our app responded successfully.
    assert!(response.status().is_success());
    // Crucially, verify that the mock embedding service was called exactly once.
    embedding_mock.assert();

    Ok(())
}
