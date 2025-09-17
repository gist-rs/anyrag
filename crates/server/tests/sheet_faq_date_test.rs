//! # Sheet Ingestion E2E Test (Modern YAML Pipeline)
//!
//! This test verifies the modern `POST /ingest/sheet` workflow. It ensures that
//! content from a sheet (served as CSV) is processed through the standard
//! knowledge ingestion pipeline, resulting in structured YAML, which is then
//! used for a RAG query.

mod common;

use anyhow::Result;
use anyrag_server::types::ApiResponse;
use common::{generate_jwt, TestApp};
use httpmock::Method;
use serde_json::{json, Value};
use turso::{params, Builder, Value as TursoValue};

#[tokio::test]
async fn test_sheet_ingestion_yaml_workflow() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn("test_sheet_ingestion_yaml_workflow").await?;
    let token = generate_jwt("sheet-ingest-user@example.com")?;

    // The raw data the mock server will provide, simulating a downloaded Google Sheet.
    let csv_content = "question,answer,release_date\nWhat is the new feature?,It is the flux capacitor.,2025-10-21";

    // The expected structured YAML that the AI should produce from the raw content.
    let expected_yaml = r#"
sections:
  - title: "Sheet Data"
    faqs:
      - question: "What is the new feature?"
        answer: "It is the flux capacitor."
"#;
    let final_rag_answer = "The new feature is the flux capacitor.";

    // --- 2. Mock External Services ---

    // A. Mock the server that provides the CSV data.
    let sheet_serve_mock = app.mock_server.mock(|when, then| {
        when.method(Method::GET)
            .path_contains("/export") // Match the Google Sheets export URL pattern
            .query_param("format", "csv");
        then.status(200)
            .header("Content-Type", "text/csv")
            .body(csv_content);
    });

    // B. Mock the LLM Restructuring call (CSV content -> structured YAML).
    let restructuring_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/test_sheet_ingestion_yaml_workflow/v1/chat/completions")
            .body_contains("expert document analyst and editor"); // KNOWLEDGE_RESTRUCTURING_SYSTEM_PROMPT
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": expected_yaml}}]}),
        );
    });

    // C. Mock the Metadata Extraction call.
    let metadata_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/test_sheet_ingestion_yaml_workflow/v1/chat/completions")
            .body_contains("extract Category, Keyphrases, and Entities");
        then.status(200)
            .json_body(json!({"choices": [{"message": {"role": "assistant", "content": "[]"}}]}));
    });

    // D. Mock the Embedding API.
    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/test_sheet_ingestion_yaml_workflow/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.3, 0.2, 0.1] }] }));
    });

    // E. Mock the RAG Query Analysis.
    let query_analysis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/test_sheet_ingestion_yaml_workflow/v1/chat/completions")
            .body_contains("expert query analyst");
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": json!({
                "entities": ["flux capacitor"], "keyphrases": ["new feature"]
            }).to_string()}}]}),
        );
    });

    // F. Mock the final RAG Synthesis call.
    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/test_sheet_ingestion_yaml_workflow/v1/chat/completions")
            .body_contains("strict, factual AI")
            .body_contains("## Sheet Data");
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_rag_answer}}]}),
        );
    });

    // --- 3. Act: Ingest the Sheet ---
    let mock_sheet_url = format!(
        "{}//spreadsheets/d/mock_sheet_id_12345/edit",
        app.mock_server.base_url()
    );

    let ingest_response = app
        .client
        .post(format!("{}/ingest/sheet", app.address)) // Endpoint no longer needs `?faq=true`
        .bearer_auth(token.clone())
        .json(&json!({ "url": mock_sheet_url }))
        .send()
        .await?
        .error_for_status()?;

    let ingest_body: ApiResponse<Value> = ingest_response.json().await?;
    assert_eq!(ingest_body.result["ingested_faqs"], 1);

    // --- 4. Assert Database State ---
    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;
    let mut stmt = conn
        .prepare("SELECT content FROM documents WHERE source_url LIKE ?")
        .await?;
    let mut rows = stmt.query(params!["%mock_sheet_id_12345%"]).await?;
    let row = rows
        .next()
        .await?
        .expect("Document for sheet not found in DB");
    let stored_content: String = match row.get_value(0)? {
        TursoValue::Text(s) => s,
        _ => panic!("Content was not a string"),
    };

    assert_eq!(
        stored_content.trim(),
        expected_yaml.trim(),
        "Stored content should be the structured YAML"
    );

    // --- 5. Act: Embed and Search ---
    app.client
        .post(format!("{}/embed/new", app.address))
        .json(&json!({ "limit": 10 }))
        .send()
        .await?
        .error_for_status()?;

    let search_response = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .bearer_auth(token)
        .json(&json!({ "query": "What is the new feature?" }))
        .send()
        .await?
        .error_for_status()?;

    let search_body: ApiResponse<Value> = search_response.json().await?;
    assert_eq!(search_body.result["text"], final_rag_answer);

    // --- 7. Assert All Mocks Were Called ---
    sheet_serve_mock.assert();
    restructuring_mock.assert();
    metadata_mock.assert();
    embedding_mock.assert_hits(2);
    query_analysis_mock.assert();
    rag_synthesis_mock.assert();

    Ok(())
}
