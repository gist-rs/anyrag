//! # Server Endpoint Tests
//!
//! This file contains integration tests for the `anyrag-server` endpoints,
//! including health checks and error handling for invalid input.

use anyhow::Result;
use reqwest::Client;
use serde_json::json;
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};

// By including the binary's main source file, we can access its public functions
// and modules as if they were part of a library. This is a common pattern for
// testing the components of a Rust binary crate.
#[path = "../src/main.rs"]
mod main;

/// Spawns the application in the background for testing.
async fn spawn_app() -> Result<String> {
    // Load environment variables from a .env file, if it exists.
    dotenvy::dotenv().ok();

    // Attempt to initialize the tracing subscriber. Ignore errors if it's
    // already been set up by another test.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .compact()
        .try_init();

    // Load the application configuration.
    let config = main::config::get_config().expect("Failed to load test configuration");

    // Bind to a random available port on 127.0.0.1.
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind random port");

    // Get the port number and build the server address string.
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{port}");

    // Spawn the server in a new tokio task, passing the listener and config.
    tokio::spawn(async move {
        if let Err(e) = main::run(listener, config).await {
            eprintln!("Server error during test: {e}");
        }
    });

    // Give the server a moment to start up.
    sleep(Duration::from_millis(100)).await;

    Ok(address)
}

#[tokio::test]
async fn test_root_and_health_check_endpoints() -> Result<()> {
    // Arrange
    let address = spawn_app().await?;
    let client = Client::new();

    // --- Test Root Endpoint ---
    let root_response = client
        .get(format!("{address}/"))
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
    let health_response = client
        .get(format!("{address}/health"))
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
    let address = spawn_app().await?;
    let client = Client::new();
    // This JSON is syntactically invalid (missing closing brace).
    let malformed_body = r#"{"prompt": "Count the corpus""#;

    // Act
    let response = client
        .post(format!("{address}/prompt"))
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
    let address = spawn_app().await?;
    let client = Client::new();
    // This JSON is syntactically valid but semantically incorrect
    // because it's missing the required `prompt` field.
    let invalid_payload = json!({
        "table_name": "bigquery-public-data.samples.shakespeare"
    });

    // Act
    let response = client
        .post(format!("{address}/prompt"))
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
