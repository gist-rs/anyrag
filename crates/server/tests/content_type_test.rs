//! # Content-Type Prompt Selection Tests
//!
//! This file tests that the server correctly uses specialized prompts when a
//! `content_type` is provided in the API request. It verifies that the
//! prompt selection logic flows correctly from the server to the library.

mod common;

use anyhow::Result;
use anyrag::prompts::tasks::{RSS_SUMMARIZATION_SYSTEM_PROMPT, RSS_SUMMARIZATION_USER_PROMPT};
use anyrag_server::config::AppConfig;
use anyrag_server::{config, state};
use common::TestApp;
use httpmock::{Method, MockServer};
use serde_json::json;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[tokio::test]
async fn test_prompt_selects_rss_template_for_rss_content_type() -> Result<()> {
    // --- 1. Arrange ---
    // This test requires a custom configuration to explicitly enable and define the
    // `rss_summarization` task, ensuring it points to our mock server. This makes
    // the test self-contained and independent of default feature flags.

    // A. Create a mock server for the AI provider.
    let mock_server = MockServer::start();

    // B. Create a temporary directory and files for a custom configuration.
    let temp_dir = tempdir()?;
    let config_path = temp_dir.path().join("config.yml");
    let db_path = temp_dir.path().join("test.db");

    // C. Define the custom configuration content.
    // We create a dedicated 'mock_provider' and assign the 'rss_summarization' task to it.
    let config_content = format!(
        r#"
db_url: "{}"
embedding:
  api_url: "http://dummy.com/embeddings"
  model_name: "mock-embedding-model"
providers:
  mock_provider:
    provider: "local"
    api_url: "{}"
    api_key: null
    model_name: "mock-rss-model"
tasks:
  rss_summarization:
    provider: "mock_provider"
    system_prompt: "{}"
    user_prompt: "{}"
"#,
        db_path.to_str().unwrap(),
        mock_server.url("/v1/chat/completions"),
        RSS_SUMMARIZATION_SYSTEM_PROMPT, // Use the real prompt from the library
        RSS_SUMMARIZATION_USER_PROMPT
    );

    let mut file = File::create(&config_path)?;
    file.write_all(config_content.as_bytes())?;

    // D. Build a custom AppState with this configuration.
    let config: AppConfig = config::get_config(Some(config_path.to_str().unwrap()))?;
    let app_state = state::build_app_state(config).await?;

    // E. Spawn the test application with our custom state and mock server.
    let app = TestApp::spawn_with_state(app_state, mock_server).await?;

    // F. Define the mock expectation for the AI call.
    let rss_system_prompt_snippet =
        "specializes in analyzing and summarizing content from RSS feeds";
    let mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains(rss_system_prompt_snippet);
        then.status(200)
            .json_body(json!({"choices": [{"message": {"role": "assistant", "content": "OK"}}]}));
    });

    // --- 2. Act ---
    let payload = json!({
        "prompt": "Summarize the latest articles about Rust.",
        "content_type": "rss",
        "context": "<item><title>Rust 1.78</title></item><item><title>New Axum Release</title></item>"
    });

    let response = app
        .client
        .post(format!("{}/prompt", app.address))
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request.");

    // --- 3. Assert ---
    assert!(response.status().is_success());
    // This verifies that the AI provider was called with a request matching our expectations.
    mock.assert();
    Ok(())
}
