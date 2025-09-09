//! # Google Sheet Ingestion and Prompting E2E Test
//!
//! This file contains a full end-to-end integration test for the feature
//! that allows users to prompt the server with a Google Sheet URL. It verifies
//! the entire workflow from URL detection to final, formatted output.

mod common;

use anyhow::Result;
use common::TestApp;
use httpmock::Method;
use serde_json::json;
use tracing::info;
use turso::Value as TursoValue;

use anyrag_server::{handlers::PromptResponse, types::ApiResponse};

#[tokio::test]
async fn test_sheet_ingestion_and_prompting_workflow() -> Result<()> {
    // --- 1. Arrange ---
    info!("[test] Starting test_sheet_ingestion_and_prompting_workflow");
    let app = TestApp::spawn().await?;
    let sqlite_provider =
        anyrag::providers::db::sqlite::SqliteProvider::new(app.db_path.to_str().unwrap()).await?;

    let sheet_path = "/spreadsheets/d/1Upsr6r6ufkYougDFVBQOQNgNf9Syrwv2CTNhFbVNu2w/edit";
    let expected_table_name = "spreadsheets_1Upsr6r6ufkYougDFVBQOQNgNf9Syrwv2CTNhFbVNu2w";

    // --- 2. Mock Services ---
    info!("[test] Setting up mocks for external services.");
    let mock_csv_content =
        "Name,Role,Team\nAlice,Engineer,Alpha\nBob,Designer,Bravo\nCharlie,PM,Alpha";
    let download_mock = app.mock_server.mock(|when, then| {
        when.method(Method::GET)
            .path("/spreadsheets/d/1Upsr6r6ufkYougDFVBQOQNgNf9Syrwv2CTNhFbVNu2w/export")
            .query_param("format", "csv");
        then.status(200).body(mock_csv_content);
    });

    // Mock for the FIRST AI call (Query Generation).
    // We make it specific by checking for content unique to the query generation prompt.
    let query_gen_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("intelligent data assistant"); // Unique to DEFAULT_QUERY_SYSTEM_PROMPT
        then.status(200).json_body(json!({
            "choices": [{
                "message": { "role": "assistant", "content": format!("```sql\nSELECT COUNT(*) as count FROM {};\n```", expected_table_name) }
            }]
        }));
    });

    // Mock for the SECOND AI call (Response Formatting).
    // We make it specific by checking for content unique to the formatting prompt.
    let format_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("strict data processor"); // Unique to the updated DEFAULT_FORMAT_SYSTEM_PROMPT
        then.status(200).json_body(json!({
            "choices": [{
                "message": { "role": "assistant", "content": "The sheet has 3 records." }
            }]
        }));
    });
    info!("[test] Mocks configured.");

    // --- 3. Send Request ---
    let payload = json!({
        "prompt": format!("Count the records in this sheet: {}", app.mock_server.url(sheet_path)),
        "instruction": "Provide a natural language summary."
    });

    info!("[test] Sending POST request to /prompt.");
    let response = app
        .client
        .post(format!("{}/prompt", app.address))
        .json(&payload)
        .send()
        .await?
        .error_for_status()?;
    info!("[test] Received response from /prompt.");

    // --- 4. Assert Server Response ---
    info!("[test] Asserting server response.");
    let result_body: ApiResponse<PromptResponse> = response.json().await?;
    let result_str = &result_body.result.text;
    assert!(
        result_str.to_string().contains("3 records"),
        "The final response did not contain the formatted text '3 records'. Got: {result_str}"
    );
    info!("[test] Server response is correct: '{}'", result_str);

    // --- 5. Assert Database State ---
    info!("[test] Asserting database state.");
    let conn = sqlite_provider.db.connect()?;
    let mut stmt = conn
        .prepare(&format!("SELECT COUNT(*) FROM {expected_table_name}"))
        .await?;
    let mut rows = stmt.query(()).await?;
    let row = rows.next().await?.expect("COUNT(*) returned no rows");
    let count: i64 = match row.get_value(0)? {
        TursoValue::Integer(i) => i,
        _ => panic!("Expected integer for count"),
    };
    assert_eq!(count, 3, "Database count verification failed.");
    info!("[test] Database state is correct. Found {} records.", count);

    // --- 6. Assert Mocks and Shutdown ---
    info!("[test] Asserting mock calls.");
    download_mock.assert();
    query_gen_mock.assert();
    format_mock.assert();
    info!("[test] Mock calls verified.");
    info!("[test] Test assertions passed and server shut down gracefully. Test finished.");

    Ok(())
}
