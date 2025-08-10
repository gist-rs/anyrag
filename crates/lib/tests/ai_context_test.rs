//! # AI Context-Aware Tests
//!
//! This file contains integration tests that verify the AI's ability to use
//! dynamically injected context, such as the current date, to generate correct queries.

// This declaration makes the `common` module available to the tests in this file.
mod common;

use crate::common::{create_real_ai_provider, setup_tracing};
use anyrag::{providers::db::sqlite::SqliteProvider, ExecutePromptOptions, PromptClientBuilder};
use chrono::Utc;
use tracing::debug;

/// Tests the full E2E flow with a real AI for a prompt using the "TODAY" context.
/// This test requires a live internet connection and valid AI provider credentials.
#[tokio::test]
async fn test_e2e_ai_query_with_today_context() {
    setup_tracing();

    // 1. Create and fully initialize the database provider first.

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
        INSERT INTO tasks (id, description, due_date) VALUES (4, 'Plan weekend', '2025-12-31');
        "
    );
    sqlite_provider
        .initialize_with_data(&setup_sql)
        .await
        .expect("Failed to initialize database");

    // 2. Now, build the client using the pre-initialized provider.
    let client = PromptClientBuilder::default()
        .ai_provider(create_real_ai_provider())
        .storage_provider(Box::new(sqlite_provider))
        .build()
        .unwrap();

    // 3. Execute a prompt that relies on the injected TODAY context.
    let options = ExecutePromptOptions {
        prompt: "What are my tasks for today?".to_string(),
        table_name: Some("tasks".to_string()),
        instruction: Some("List the task descriptions.".to_string()),
        ..Default::default()
    };

    let result = client.execute_prompt_with_options(options).await;

    if let Err(e) = &result {
        // Use eprint to make sure the error is visible in test logs.
        eprintln!("Error in test_e2e_ai_query_with_today_context: {e}");
    }
    assert!(result.is_ok());
    let output = result.unwrap();

    debug!("E2E `TODAY` context test output: {output}");

    // 4. Assert that the response contains today's tasks but not others.
    assert!(output.contains("Team meeting"));
    assert!(output.contains("Review PRs"));
    assert!(!output.contains("Write report"));
    assert!(!output.contains("Plan weekend"));
}
