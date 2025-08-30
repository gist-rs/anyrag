//! # Content-Type Prompt Selection Tests
//!
//! This file tests that the server correctly uses specialized prompts when a
//! `content_type` is provided in the API request. It verifies that the
//! prompt selection logic flows correctly from the server to the library.

mod common;

use common::TestApp;
use httpmock::Method;
use serde_json::json;

#[tokio::test]
async fn test_prompt_selects_rss_template_for_rss_content_type() {
    // --- Arrange ---
    let app = TestApp::spawn().await.unwrap();

    // The harness already mocks the AI provider. We just need to define the
    // expected behavior for this specific test. The `TestApp`'s AI provider points to
    // the mock server, so we can set expectations on it.
    let rss_system_prompt_snippet =
        "specializes in analyzing and summarizing content from RSS feeds";

    let mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            // A snippet is used to make the test less brittle to minor prompt wording changes.
            .body_contains(rss_system_prompt_snippet);
        then.status(200)
            .json_body(json!({"choices": [{"message": {"role": "assistant", "content": "OK"}}]}));
    });

    // --- Act ---
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

    // --- Assert ---
    assert!(response.status().is_success());
    // This verifies that the AI provider was called with a request matching our expectations.
    mock.assert();
}
