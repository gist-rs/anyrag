//! # PDF Ingestion and RAG E2E Test
//!
//! This test verifies the entire end-to-end workflow for PDF ingestion and search:
//! 1. A PDF is generated and uploaded to `/ingest/pdf`.
//! 2. The server extracts the text, uses an LLM to restructure it into YAML, and extracts metadata.
//! 3. The final YAML content is stored in the database.
//! 4. The new document is embedded via a mock embedding API.
//! 5. A RAG query is performed via `/search/knowledge`, which retrieves the YAML,
//!    chunks it by section, and uses an LLM to synthesize the final answer.

mod common;

use anyhow::Result;
use anyrag_server::types::ApiResponse;
use anyrag_test_utils::helpers::generate_test_pdf;
use common::{generate_jwt, TestApp};
use httpmock::Method;
use serde_json::{json, Value};
use turso::{Builder, Value as TursoValue};

#[tokio::test]
async fn test_pdf_ingestion_and_rag_workflow() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let test_name = "test_pdf_ingestion_and_rag_workflow";
    let app = TestApp::spawn(test_name).await?;
    let token = generate_jwt("pdf-ingest-user@example.com")?;

    let pdf_content = "The magic number is 3.14159.";
    let expected_yaml = r#"
sections:
  - title: "General Information"
    faqs:
      - question: "What is the magic number?"
        answer: "The magic number is 3.14159."
"#;
    let final_rag_answer = "Based on the document, the magic number is 3.14159.";
    let pdf_data = generate_test_pdf(pdf_content)?;

    // --- 2. Mock External Services ---
    // This test requires mocking every external HTTP call the server makes.

    // A. Mock the AI call for restructuring the PDF content into YAML.
    let restructuring_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(format!("/{test_name}/v1/chat/completions"))
            .body_contains("expert document analyst and editor"); // Unique to this prompt
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": expected_yaml}}]}),
        );
    });

    // B. Mock the AI call for extracting metadata from the YAML.
    let metadata_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(format!("/{test_name}/v1/chat/completions"))
            .body_contains("You are a document analyst."); // Unique to this prompt
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": json!([
                {"type": "KEYPHRASE", "subtype": "CONCEPT", "value": "magic number"}
            ]).to_string()}}]}),
        );
    });

    // C. Mock the Embedding API (for new doc and for search query).
    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(format!("/{test_name}/v1/embeddings"));
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.1, 0.2, 0.3] }] }));
    });

    // D. Mock the RAG Query Analysis call.
    let query_analysis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(format!("/{test_name}/v1/chat/completions"))
            .body_contains("expert query analyst");
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": json!({
                "entities": [], "keyphrases": ["magic number"]
            }).to_string()}}]}),
        );
    });

    // E. Mock the final RAG Synthesis call.
    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(format!("/{test_name}/v1/chat/completions"))
            .body_contains("strict, factual AI")
            .body_contains("## General Information"); // Verify it receives the context chunked from the YAML
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_rag_answer}}]}),
        );
    });

    // --- 3. Act: Ingest the PDF ---
    let form = reqwest::multipart::Form::new()
        .part(
            "file",
            reqwest::multipart::Part::bytes(pdf_data).file_name("test.pdf"),
        )
        .part("extractor", reqwest::multipart::Part::text("local")); // Use the local PDF parser

    let ingest_res = app
        .client
        .post(app.url("/ingest/pdf"))
        .bearer_auth(token.clone())
        .multipart(form)
        .send()
        .await?
        .error_for_status()?;

    let ingest_body: ApiResponse<Value> = ingest_res.json().await?;
    assert_eq!(ingest_body.result["ingested_documents"], 1);

    // --- 4. Act: Embed the new document ---
    app.client
        .post(app.url("/embed/new"))
        .json(&json!({ "limit": 10 }))
        .send()
        .await?
        .error_for_status()?;

    // --- 5. Act: Perform a RAG search ---
    let search_res = app
        .client
        .post(app.url("/search/knowledge"))
        .bearer_auth(token)
        .json(&json!({ "query": "what is the magic number?" }))
        .send()
        .await?
        .error_for_status()?;

    // --- 6. Assert API and DB State ---
    let search_body: ApiResponse<Value> = search_res.json().await?;
    assert_eq!(search_body.result["text"], final_rag_answer);

    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;
    let mut stmt = conn
        .prepare("SELECT content FROM documents WHERE source_url = 'test.pdf'")
        .await?;
    let mut rows = stmt.query(()).await?;
    let row = rows.next().await?.expect("Document not found in DB");
    let stored_content: String = match row.get_value(0)? {
        TursoValue::Text(s) => s,
        _ => panic!("Content was not a string"),
    };
    assert_eq!(stored_content.trim(), expected_yaml.trim());

    // --- 7. Assert All Mocks Were Called Correctly ---
    restructuring_mock.assert();
    metadata_mock.assert();
    embedding_mock.assert_hits(2); // Once for new doc, once for search query
    query_analysis_mock.assert();
    rag_synthesis_mock.assert();

    Ok(())
}
