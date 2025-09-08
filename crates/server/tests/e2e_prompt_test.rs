//! # End-to-End Prompt Execution Tests

// This makes the `common` module available to this test file.
// By convention, Rust looks for `tests/common.rs` or `tests/common/mod.rs`.
mod common;

use anyhow::Result;
use common::TestApp;
use httpmock::Method;
use serde_json::json;
use turso::Builder;

#[tokio::test]
async fn test_e2e_prompt_execution() -> Result<()> {
    let app = TestApp::spawn().await?;

    // --- Arrange: Database Setup ---
    // The TestApp creates an empty database. We need to create the table
    // that this test intends to query.
    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;
    conn.execute(
        "CREATE TABLE test_table (id INTEGER PRIMARY KEY, name TEXT, value REAL);",
        (),
    )
    .await?;

    // The harness uses a mock AI provider. We need to mock the two calls the
    // /prompt endpoint will make: one for query generation, one for formatting.

    // 1. Mock the Query Generation call.
    // The test harness uses an empty SQLite database. The mock query must be
    // valid SQLite that can execute successfully against this empty database.
    // A simple SELECT of a constant value works perfectly.
    let query_gen_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("intelligent data assistant"); // Differentiate from the format call
        then.status(200).json_body(
            // Return a valid SQLite query. The alias is important so the
            // JSON result sent to the next step is predictable.
            json!({"choices": [{"message": {"role": "assistant", "content": "SELECT 27894 AS total;"}}]}),
        );
    });

    // 2. Mock the Response Formatting call.
    // This mock simulates the AI formatting the raw JSON result from the database
    // (`[{"total":27894}]`) into a human-readable string.
    let format_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("strict data processor"); // Differentiate from query gen
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": "27,894"}}]}),
        );
    });

    // The `table_name` is required to trigger the "Text-to-SQL" logic branch in the
    // prompt client. Crucially, the name must be a valid identifier for the underlying
    // database (SQLite in this test), as the provider will try to look up its schema.
    // A name with dots like "a.b.c" is invalid in SQLite and caused the original failure.
    let payload = json!({
        "prompt": "What is the total word_count for the corpus 'kinghenryv'?",
        "table_name": "test_table", // Use a valid SQLite table name
        "instruction": "Answer with only the number, with thousand format."
    });

    let response = app
        .client
        .post(format!("{}/prompt?debug=true", app.address))
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(
        response.status().is_success(),
        "Request failed with status: {}. Body: {:?}",
        response.status(),
        response.text().await
    );

    let body: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse response JSON");

    let result = body["result"]["text"].as_str().unwrap();

    assert!(body["debug"].is_object(), "Debug field should be present");
    assert!(
        result.contains("27,894"),
        "Response did not contain the expected result."
    );

    // Verify both mocks were called
    query_gen_mock.assert();
    format_mock.assert();

    Ok(())
}
