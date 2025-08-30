//! # Text Ingest Endpoint Tests
//!
//! This file contains integration tests for the `POST /ingest/text` endpoint.
//! It verifies that the server can accept raw text, chunk it correctly
//! according to the library's logic, and store it in the database.

mod common;

use anyhow::Result;
use common::TestApp;
use serde_json::json;
use turso::Value as TursoValue;

#[tokio::test]
async fn test_ingest_text_endpoint_success() -> Result<()> {
    // --- Arrange ---

    let app = TestApp::spawn().await?;

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

    // Check the total count of articles.
    let mut result_set = conn
        .query("SELECT COUNT(*) FROM articles", ())
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
    assert_eq!(count, 3, "The number of articles in the DB should be 3.");

    // Check the content of the first chunk (the short paragraph).
    let mut result_set = conn
        .query(
            "SELECT description FROM articles WHERE source_url = 'manual_test' ORDER BY id ASC LIMIT 1",
            (),
        )
        .await
        .expect("Failed to query db for specific article");
    let row = result_set
        .next()
        .await
        .expect("Failed to get row for article 1")
        .expect("Row for article 1 is None");
    let description: String = match row.get_value(0).unwrap() {
        TursoValue::Text(s) => s,
        other => panic!("Expected Text for description, got {other:?}"),
    };
    assert_eq!(description, "This is the first paragraph.");

    Ok(())
}
