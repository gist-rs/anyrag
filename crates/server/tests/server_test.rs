//! # Server Endpoint Tests
//!
//! This file contains integration tests for the `anyrag-server` endpoints,
//! including health checks and error handling for invalid input.

mod common;

use anyhow::Result;
use common::TestApp;
use httpmock::Method;
use serde_json::json;

#[tokio::test]
async fn test_root_and_health_check_endpoints() -> Result<()> {
    // Arrange
    let app = TestApp::spawn("test_root_and_health_check_endpoints").await?;

    // --- Test Root Endpoint ---
    let root_response = app
        .client
        .get(format!("{}/", app.address))
        .send()
        .await
        .expect("Failed to execute request to /");

    // Assert
    assert!(root_response.status().is_success());
    assert_eq!(
        "anyrag server is running.",
        root_response.text().await.unwrap()
    );

    // --- Test Health Check Endpoint ---
    let health_response = app
        .client
        .get(format!("{}/health", app.address))
        .send()
        .await
        .expect("Failed to execute request to /health");

    // Assert
    assert!(health_response.status().is_success());
    assert_eq!("OK", health_response.text().await.unwrap());

    Ok(())
}

#[tokio::test]
async fn test_prompt_handler_malformed_json() -> Result<()> {
    // Arrange
    let app = TestApp::spawn("test_prompt_handler_malformed_json").await?;
    // This JSON is syntactically invalid (missing closing brace).
    let malformed_body = r#"{"prompt": "Count the corpus""#;

    // Act
    let response = app
        .client
        .post(format!("{}/prompt", app.address))
        .header("Content-Type", "application/json")
        .body(malformed_body)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    // Axum's `Json` extractor should reject malformed JSON with a 400 Bad Request.
    assert_eq!(400, response.status().as_u16());

    Ok(())
}

#[tokio::test]
async fn test_prompt_handler_invalid_payload() -> Result<()> {
    // Arrange
    let app = TestApp::spawn("test_prompt_handler_invalid_payload").await?;
    // This test triggers an internal server error before any AI call is made,
    // but the app startup still requires the mock provider URL to be valid.
    // A placeholder mock prevents panics.
    app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/test_prompt_handler_invalid_payload/v1/chat/completions");
        then.status(200)
            .json_body(json!({"choices": [{"message": {"role": "assistant", "content": "OK"}}]}));
    });
    // This JSON is syntactically valid but semantically incorrect
    // because it's missing the required `prompt` field.
    let invalid_payload = json!({
        "table_name": "bigquery-public-data.samples.shakespeare"
    });

    // Act
    let response = app
        .client
        .post(format!("{}/prompt", app.address))
        .json(&invalid_payload)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    // The deserialization into `ExecutePromptOptions` happens inside the handler,
    // and the resulting `serde_json::Error` is currently mapped to a 500 error
    // via `PromptError::JsonSerialization`. This test confirms that behavior.
    assert_eq!(500, response.status().as_u16());
    let body: serde_json::Value = response.json().await?;
    let error_message = body["error"].as_str().unwrap();
    assert!(error_message.contains("Failed to serialize result: missing field `prompt`"));

    Ok(())
}
