//! # AI Context-Aware Tests
//!
//! This file contains integration tests that verify the AI's ability to use
//! dynamically injected context, such as the current date, to generate correct queries.

// This declaration makes the `common` module available to the tests in this file.
mod common;

use crate::common::{setup_tracing, MockAiProvider};
use anyrag::{providers::db::sqlite::SqliteProvider, ExecutePromptOptions, PromptClientBuilder};
use chrono::{DateTime, Utc};
use tracing::debug;

/// Tests that the prompt client correctly injects the "TODAY" context into the prompt
/// sent to the AI provider. It uses a mock AI to isolate this logic.
#[tokio::test]
async fn test_ai_prompt_construction_with_today_context() {
    setup_tracing();

    // 1. Arrange: Setup the database and mock AI provider.
    let sqlite_provider = SqliteProvider::new(":memory:")
        .await
        .expect("Failed to create SqliteProvider");

    let today_str = Utc::now().format("%Y-%m-%d").to_string();
    let setup_sql = format!(
        "
        CREATE TABLE tasks (id INTEGER PRIMARY KEY, description TEXT, due_date TEXT);
        INSERT INTO tasks (id, description, due_date) VALUES (1, 'Write report', '2024-01-01');
        INSERT INTO tasks (id, description, due_date) VALUES (2, 'Team meeting', '{today_str}');
        INSERT INTO tasks (id, description, due_date) VALUES (3, 'Review PRs', '{today_str}');
        "
    );
    sqlite_provider
        .initialize_with_data(&setup_sql)
        .await
        .expect("Failed to initialize database");

    // The mock AI will return a query that uses the exact date format we expect.
    // The second response is for the formatting step.
    let mock_responses = vec![
        format!("SELECT description FROM tasks WHERE due_date = '{today_str}'"),
        "Your tasks for today are: Team meeting, and Review PRs.".to_string(),
    ];
    let mock_ai_provider = MockAiProvider::new(mock_responses);
    let call_history = mock_ai_provider.call_history.clone();

    let client = PromptClientBuilder::default()
        .ai_provider(Box::new(mock_ai_provider))
        .storage_provider(Box::new(sqlite_provider))
        .build()
        .unwrap();

    // 2. Act: Execute a prompt that relies on the injected TODAY context.
    let options = ExecutePromptOptions {
        prompt: "What are my tasks for today?".to_string(),
        table_name: Some("tasks".to_string()),
        instruction: Some("List the task descriptions.".to_string()),
        ..Default::default()
    };

    let result = client.execute_prompt_with_options(options).await;
    assert!(
        result.is_ok(),
        "Prompt execution failed: {:?}",
        result.err()
    );
    let output = result.unwrap();

    debug!("Mock `TODAY` context test output: {output:?}");

    // 3. Assert: Check both the final output and the prompt that was constructed.
    assert!(output.text.contains("Team meeting"));
    assert!(output.text.contains("Review PRs"));
    assert!(!output.text.contains("Write report"));

    let history = call_history.read().unwrap();
    assert_eq!(history.len(), 2, "Expected two calls to the AI provider");

    let (_system_prompt, user_prompt) = &history[0];

    // --- Assert that the context was correctly injected ---
    assert!(
        user_prompt.contains("# TODAY\n"),
        "User prompt should contain the '# TODAY' section"
    );

    // Find the UTC date line and parse it to verify it's a valid, recent timestamp.
    let utc_line = user_prompt
        .lines()
        .find(|line| line.starts_with("UTC: "))
        .expect("Could not find UTC date line in prompt");

    let utc_date_str = utc_line.trim_start_matches("UTC: ").trim();
    let parsed_utc_date =
        DateTime::parse_from_rfc3339(utc_date_str).expect("Failed to parse UTC date from prompt");
    let now = Utc::now();
    let duration_since_prompt = now.signed_duration_since(parsed_utc_date.with_timezone(&Utc));

    // The prompt should have been generated within the last 5 seconds.
    assert!(
        duration_since_prompt.num_seconds() < 5,
        "The timestamp in the prompt context is too old."
    );
}
