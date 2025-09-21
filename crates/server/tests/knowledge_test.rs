//! # Knowledge Base Pipeline Integration Test (Refactored)
//!
//! This file tests the new, refactored knowledge base ingestion pipeline.
//! It verifies the end-to-end flow from fetching a web page to storing structured
//! YAML in the database and then correctly exporting it for fine-tuning.

mod common;

use anyhow::Result;
use anyrag::{
    ingest::{knowledge::export_for_finetuning, IngestionPrompts, Ingestor},
    providers::{ai::local::LocalAiProvider, db::sqlite::SqliteProvider},
};
use anyrag_web::{WebIngestStrategy, WebIngestor};
use httpmock::{Method, MockServer};
use serde_json::{json, Value};
use tempfile::NamedTempFile;
use tracing::info;
use turso::Value as TursoValue;

#[tokio::test]
#[ignore]
async fn test_new_knowledge_ingestion_and_export_pipeline() -> Result<()> {
    // --- 1. Arrange ---
    let mock_server = MockServer::start();
    let db_file = NamedTempFile::new()?;
    let db_path = db_file.path();

    // Initialize the provider and ensure the schema exists.
    let sqlite_provider = SqliteProvider::new(db_path.to_str().unwrap()).await?;
    sqlite_provider.initialize_schema().await?;
    let db = &sqlite_provider.db;

    let page_url = mock_server.url("/test-page");

    // A. Mock the source web page content.
    let html_content = "<html><head><title>Test Page</title></head><body><h1>Main Title</h1><p>Q: What is this?</p><p>A: It is a test.</p></body></html>";
    let source_mock = mock_server.mock(|when, then| {
        when.method(Method::GET).path("/test-page");
        then.status(200).body(html_content);
    });

    // B. Mock the LLM Restructuring call.
    // The AI provider will receive markdown converted from the HTML above.
    // It should respond with the structured YAML.
    let expected_yaml_output = r#"
sections:
  - title: "Main Title"
    faqs:
      - question: "What is this?"
        answer: "It is a test."
"#;
    // This mock is for the restructuring call
    let llm_restructure_mock = mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            // The key part of the converted markdown to expect.
            .body_contains("Q: What is this?");
        then.status(200)
            .json_body(json!({"choices": [{"message": {"role": "assistant", "content": expected_yaml_output}}]}));
    });

    // C. Mock the Metadata Extraction call (it will be called after restructuring)
    let metadata_response =
        json!([{"type": "KEYPHRASE", "subtype": "CONCEPT", "value": "testing"}]);
    let llm_metadata_mock = mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            // Match based on the YAML content being sent for metadata extraction
            .body_contains("faqs:");
        then.status(200)
            .json_body(json!({"choices": [{"message": {"role": "assistant", "content": metadata_response.to_string()}}]}));
    });

    // D. Setup the AI provider to point to our mock server.
    let ai_provider = LocalAiProvider::new(mock_server.url("/v1/chat/completions"), None, None)?;

    // --- 2. Act ---
    // Run the entire ingestion pipeline.
    let prompts = IngestionPrompts {
        restructuring_system_prompt: "You are an expert document analyst.",
        metadata_extraction_system_prompt: "You are an expert metadata extractor.",
    };

    // Instantiate the ingestor plugin.
    let ingestor = WebIngestor::new(db, &ai_provider, prompts);

    // Serialize the source information into the JSON format expected by the ingestor.
    let source_json = json!({
        "url": page_url,
        "strategy": WebIngestStrategy::RawHtml
    })
    .to_string();

    // Call the generic ingest method.
    let ingest_result = ingestor.ingest(&source_json, None).await?;
    let ingested_count = ingest_result.documents_added;

    // --- 3. Assert Database State ---
    info!(
        "Ingestion pipeline finished. Ingested {} documents.",
        ingested_count
    );
    assert_eq!(
        ingested_count, 1,
        "Expected the pipeline to process 1 document."
    );

    // Verify mocks were called as expected.
    source_mock.assert();
    llm_restructure_mock.assert();
    llm_metadata_mock.assert();

    let conn = db.connect()?;
    let mut stmt = conn
        .prepare("SELECT content FROM documents WHERE source_url = ?")
        .await?;
    let mut rows = stmt.query(vec![TursoValue::Text(page_url)]).await?;

    let row = rows
        .next()
        .await?
        .expect("Expected to find the ingested document in the database.");

    let stored_content = match row.get_value(0)? {
        TursoValue::Text(s) => s,
        v => panic!("Expected text for content, got {v:?}"),
    };

    // The stored content should be the clean YAML from the LLM.
    assert_eq!(stored_content.trim(), expected_yaml_output.trim());
    info!("-> Database state verified successfully. Stored content is correct.");

    // --- 4. Act & Assert: Export for Fine-Tuning ---
    let export_body = export_for_finetuning(db).await?;
    let lines: Vec<&str> = export_body.trim().lines().collect();
    assert_eq!(lines.len(), 1, "Expected one line in the JSONL output.");

    let json: Value = serde_json::from_str(lines[0])?;
    let question = json["messages"][1]["content"].as_str().unwrap();
    let answer = json["messages"][2]["content"].as_str().unwrap();

    assert_eq!(question, "What is this?");
    assert_eq!(answer, "It is a test.");

    info!("-> Fine-tuning export verified successfully.");

    Ok(())
}
