//! # Sheet Ingest Endpoint Tests
//!
//! This file contains integration tests for the `POST /ingest/sheet` endpoint.
//! It verifies that the server can accept a Google Sheet URL, fetch its content,
//! use an AI service to restructure it and extract metadata, and then store the
//! final structured data in the database.

mod common;

use anyhow::Result;
use anyrag::prompts::{knowledge, tasks};
use common::TestApp;
use httpmock::Method;
use serde_json::json;
use turso::params;

use crate::common::generate_jwt;

#[tokio::test]
async fn test_ingest_sheet_endpoint_success() -> Result<()> {
    // --- Arrange ---

    let test_case_name = "test_ingest_sheet_endpoint_success";
    let app = TestApp::spawn(test_case_name).await?;
    let user_identifier = "ingest-sheet-user@example.com";
    let token = generate_jwt(user_identifier)?;

    // --- Mock Data ---

    let csv_content = "question,answer\nWhat is the new feature?,It is the flux capacitor.";
    let expected_yaml = r#"
sections:
  - title: "Sheet Data"
    faqs:
      - question: "What is the new feature?"
        answer: "It is the flux capacitor."
"#;
    let mock_metadata = json!([
        {
            "type": "KEYPHRASE",
            "subtype": "CONCEPT",
            "value": "flux capacitor"
        },
        {
            "type": "CATEGORY",
            "subtype": "CONCEPT",
            "value": "Fictional Technology"
        }
    ]);

    // --- Mock External Services ---

    let restructure_payload = json!({
        "model": "mock-gemini-model",
        "messages": [
            {"role": "system", "content": knowledge::KNOWLEDGE_RESTRUCTURING_SYSTEM_PROMPT},
            {"role": "user", "content": "# Markdown Content to Process:\nquestion,answer\nWhat is the new feature?,It is the flux capacitor."},
        ],
        "temperature": 0.0,
        "max_tokens": 8192,
        "stream": false
    });

    let metadata_payload = json!({
        "model": "mock-gemini-model",
        "messages": [
            {"role": "system", "content": tasks::KNOWLEDGE_METADATA_EXTRACTION_SYSTEM_PROMPT},
            {"role": "user", "content": expected_yaml.trim()},
        ],
        "temperature": 0.0,
        "max_tokens": 8192,
        "stream": false
    });

    // 1. Mock the Google Sheet CSV download.
    let sheet_mock = app.mock_server.mock(|when, then| {
        when.method(Method::GET)
            .path("/spreadsheets/d/mock_sheet_id_12345/export")
            .query_param("format", "csv");
        then.status(200)
            .header("Content-Type", "text/csv")
            .body(csv_content);
    });

    // 2. Mock the first AI call (restructuring CSV to YAML).
    let restructure_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(format!("/{test_case_name}/v1/chat/completions"))
            .json_body(restructure_payload);
        then.status(200).json_body(json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": expected_yaml
                }
            }]
        }));
    });

    // 3. Mock the second AI call (extracting metadata from YAML).
    let metadata_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(format!("/{test_case_name}/v1/chat/completions"))
            .json_body(metadata_payload);
        then.status(200).json_body(json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": mock_metadata.to_string()
                }
            }]
        }));
    });

    // --- Prepare Request Payload ---

    let mock_sheet_url = format!(
        "{}/spreadsheets/d/mock_sheet_id_12345/edit",
        app.mock_server.base_url()
    );
    let payload = json!({
        "url": mock_sheet_url,
    });

    // --- Act ---

    let response = app
        .client
        .post(format!("{}/ingest/sheet", app.address))
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
    // The entire sheet is one document.
    assert_eq!(response_body["result"]["ingested_chunks"], 1);
    let doc_id = response_body["result"]["document_ids"][0]
        .as_str()
        .expect("Document ID is not a string");

    // --- Assert (Database State) ---
    let db = turso::Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;

    // A. Assert that the document content is the structured YAML.
    let stored_content: String = conn
        .query(
            "SELECT content FROM documents WHERE id = ?",
            params![doc_id],
        )
        .await?
        .next()
        .await?
        .expect("Document not found in DB")
        .get(0)?;
    assert_eq!(stored_content.trim(), expected_yaml.trim());

    // B. Assert that the metadata was extracted and stored correctly.
    let mut stmt_meta = conn
        .prepare("SELECT metadata_type, metadata_value FROM content_metadata WHERE document_id = ? ORDER BY metadata_type")
        .await?;
    let mut rows_meta = stmt_meta.query(params![doc_id]).await?;

    let row1 = rows_meta.next().await?.expect("Expected metadata row 1");
    assert_eq!(row1.get::<String>(0)?, "CATEGORY");
    assert_eq!(row1.get::<String>(1)?, "Fictional Technology");

    let row2 = rows_meta.next().await?.expect("Expected metadata row 2");
    assert_eq!(row2.get::<String>(0)?, "KEYPHRASE");
    assert_eq!(row2.get::<String>(1)?, "flux capacitor");

    assert!(
        rows_meta.next().await?.is_none(),
        "Found more metadata than expected"
    );

    // --- Assert Mocks Were Called ---
    sheet_mock.assert();
    restructure_mock.assert();
    metadata_mock.assert();

    Ok(())
}
