//! # Text Ingest Endpoint Tests
//!
//! This file contains integration tests for the `POST /ingest/text` endpoint.
//! It verifies that the server can accept raw text, chunk it correctly
//! according to the library's logic, and store it in the database.

mod common;

use anyhow::Result;
use common::TestApp;
use httpmock::Method;
use serde_json::json;
use turso::Value as TursoValue;

use crate::common::generate_jwt;

#[tokio::test]
async fn test_ingest_text_endpoint_success() -> Result<()> {
    // --- Arrange ---

    let app = TestApp::spawn("test_ingest_text_endpoint_success").await?;
    let user_identifier = "ingest-text-user@example.com";
    let token = generate_jwt(user_identifier)?;

    // This test doesn't call the AI, but a placeholder mock is needed for stable app startup.
    app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/test_ingest_text_endpoint_success/v1/chat/completions");
        then.status(200)
            .json_body(json!({"choices": [{"message": {"role": "assistant", "content": "OK"}}]}));
    });

    // 3. Define the text payload. It includes a short paragraph and one that
    // will be split into two chunks by the chunking logic (4096 limit).
    let long_paragraph = "a".repeat(5000);
    let text_to_ingest = format!("This is the first paragraph.\n\n{long_paragraph}");
    let payload = json!({
        "text": text_to_ingest,
        "source": "manual_test"
    });

    // --- Act ---

    // 4. Call the /ingest/text endpoint on our app server.
    let response = app
        .client
        .post(format!("{}/ingest/text", app.address))
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request.");

    // --- Assert (API Response) ---
    assert!(
        response.status().is_success(),
        "Request failed with status: {}",
        response.status()
    );
    let response_body: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse response JSON");
    // We expect 3 chunks: one for the short paragraph, two for the long one.
    assert_eq!(response_body["result"]["ingested_chunks"], 3);

    // --- Assert (Database State) ---
    // Verify the data was written to the database correctly.
    let db = turso::Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await
        .expect("Failed to connect to temp db");
    let conn = db.connect().expect("Failed to get connection from db");

    // Check the total count of documents.
    let mut result_set = conn
        .query("SELECT COUNT(*) FROM documents", ())
        .await
        .expect("Failed to query db for count");
    let row = result_set
        .next()
        .await
        .expect("Failed to get next row")
        .expect("Row is None");

    let count: i64 = match row.get_value(0).unwrap() {
        TursoValue::Integer(i) => i,
        other => panic!("Expected Integer, got {other:?}"),
    };
    assert_eq!(count, 3, "The number of documents in the DB should be 3.");

    // Check the content of the first chunk (the short paragraph).
    let mut result_set = conn
        .query(
            "SELECT content FROM documents WHERE source_url = 'manual_test#chunk_0' ORDER BY id ASC LIMIT 1",
            (),
        )
        .await
        .expect("Failed to query db for specific document");
    let row = result_set
        .next()
        .await
        .expect("Failed to get row for document 1")
        .expect("Row for document 1 is None");
    let content: String = match row.get_value(0).unwrap() {
        TursoValue::Text(s) => s,
        other => panic!("Expected Text for content, got {other:?}"),
    };
    assert_eq!(content, "This is the first paragraph.");

    Ok(())
}
