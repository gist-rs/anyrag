//! # Prompt Execution Tests with SQLite
//!
//! This test suite validates the `PromptClient`'s ability to execute
//! prompts against a non-BigQuery storage backend (SQLite), ensuring
//! that the `Storage` trait is properly decoupled.

mod common;

use crate::common::TestSetup;
use anyrag::{
    providers::ai::local::LocalAiProvider,
    types::{ExecutePromptOptions, PromptClientBuilder},
};
use httpmock::{Method, MockServer};
use serde_json::json;

#[tokio::test]
async fn test_prompt_execution_with_sqlite() {
    // --- Setup ---
    // Spin up a test setup with an in-memory SQLite DB.
    let test_setup = TestSetup::new().await;
    // Spin up a mock server to simulate AI provider responses.
    let mock_server = MockServer::start();

    // --- Mock AI Responses ---
    // 1. Mock the response for the query generation stage.
    let query_gen_mock = mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("expert for SQL"); // The prompt for SQLite uses "SQL"
        then.status(200).json_body(
            // The AI's "response" is the generated SQL query.
            json!({"choices": [{"message": {"role": "assistant", "content": "SELECT name FROM test_table WHERE id = 1;"}}]}),
        );
    });

    // 2. Mock the response for the final formatting stage.
    let format_mock = mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("strict data processor");
        then.status(200).json_body(
            // The AI's "response" is the final, user-facing text.
            json!({"choices": [{"message": {"role": "assistant", "content": "The name for ID 1 is 'test'."}}]}),
        );
    });

    // --- Client Configuration ---
    // Configure an AI provider to point to our mock server.
    let ai_provider = Box::new(
        LocalAiProvider::new(mock_server.url("/v1/chat/completions"), None, None)
            .expect("Failed to create LocalAiProvider"),
    );

    // Build the PromptClient, injecting the mock AI provider and the
    // SQLite storage provider from our test setup.
    let client = PromptClientBuilder::new()
        .ai_provider(ai_provider)
        .storage_provider(test_setup.storage_provider)
        .build()
        .expect("Failed to build PromptClient");

    // --- Execute Prompt ---
    let options = ExecutePromptOptions {
        prompt: "What is the name for ID 1?".to_string(),
        table_name: Some("test_table".to_string()),
        instruction: Some("Tell me the name.".to_string()),
        ..Default::default()
    };

    let result = client
        .execute_prompt_with_options(options)
        .await
        .expect("Prompt execution failed");

    // --- Assertions ---
    // Verify that the final text matches the AI's formatted response.
    assert_eq!(result.text, "The name for ID 1 is 'test'.");

    // Verify that the generated SQL is what we expected from the first mock.
    assert_eq!(
        result.generated_sql,
        Some("SELECT name FROM test_table WHERE id = 1;".to_string())
    );

    // Verify that the raw database result from SQLite is correct.
    assert_eq!(
        result.database_result,
        Some("[{\"name\":\"test\"}]".to_string())
    );

    // Ensure that both mock endpoints were called exactly once.
    query_gen_mock.assert();
    format_mock.assert();
}
