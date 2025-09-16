//! # PDF Ingestion and RAG E2E Test (YAML Pipeline)
//!
//! This test verifies the entire `POST /ingest/pdf` workflow based on the new,
//! structured YAML pipeline. It ensures that:
//! 1. A PDF is generated, and its raw text is extracted.
//! 2. The raw text is sent to a mock LLM for refinement into Markdown.
//! 3. The refined Markdown is sent to a mock LLM for restructuring into YAML.
//! 4. The final YAML content is stored as a single document in the database.
//! 5. The new document is embedded via a mock embedding API.
//! 6. A final RAG query (`/search/knowledge`) correctly chunks the YAML,
//!    retrieves the relevant context, and synthesizes the correct answer.

mod common;

use anyhow::Result;
use anyrag_server::types::ApiResponse;
use common::{generate_jwt, pdf_helper::generate_test_pdf, TestApp};
use httpmock::Method;
use serde_json::{json, Value};
use turso::{Builder, Value as TursoValue};

#[tokio::test]
async fn test_pdf_ingestion_and_rag_workflow_yaml() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn().await?;

    let messy_sentence =
        "ThisIsA   messy    sentence. It needs refinement. The magic number is 3.14159.";
    let refined_markdown =
        "- This is a messy sentence.\n- It needs refinement.\n- The magic number is 3.14159.";
    let expected_yaml = r#"
sections:
  - title: "General Information"
    faqs:
      - question: "What is the magic number?"
        answer: "The magic number is 3.14159."
"#;
    let final_rag_answer = "Based on the document, the magic number is 3.14159.";

    let pdf_data = generate_test_pdf(messy_sentence)?;

    // --- 2. Mock External Services ---
    // A. Mock LLM Refinement (raw text -> clean markdown)
    let refinement_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("expert technical analyst");
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": refined_markdown}}]}),
        );
    });

    // B. Mock LLM Restructuring (clean markdown -> structured YAML)
    let restructuring_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("expert document analyst and editor"); // Unique to the restructuring prompt
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": expected_yaml}}]}),
        );
    });

    // C. Mock Metadata Extraction (structured YAML -> metadata JSON)
    let metadata_extraction_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("You are an expert document analyst."); // Metadata prompt
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": json!([
                {"type": "KEYPHRASE", "subtype": "CONCEPT", "value": "magic number"}
            ]).to_string()}}]}),
        );
    });

    // D. Mock Embedding API
    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.1, 0.2, 0.3] }] }));
    });

    // E. Mock RAG Query Analysis
    let query_analysis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("expert query analyst");
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": json!({
                "entities": [],
                "keyphrases": ["magic number"]
            }).to_string()}}]}),
        );
    });

    // F. Mock final RAG synthesis call
    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("strict, factual AI")
            // Verify it receives the context chunked from the YAML
            .body_contains("## General Information")
            .body_contains("### Q: What is the magic number?")
            .body_contains("The magic number is 3.14159.");
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_rag_answer}}]}),
        );
    });

    // --- 3. Execute Ingestion ---
    let user_identifier = "pdf-ingest-user@example.com";
    let token = generate_jwt(user_identifier)?;

    let form = reqwest::multipart::Form::new()
        .part(
            "file",
            reqwest::multipart::Part::bytes(pdf_data).file_name("test.pdf"),
        )
        .part("extractor", reqwest::multipart::Part::text("local"));

    let ingest_res = app
        .client
        .post(format!("{}/ingest/pdf", app.address))
        .bearer_auth(token.clone())
        .multipart(form)
        .send()
        .await?
        .error_for_status()?;

    let ingest_body: ApiResponse<Value> = ingest_res.json().await?;
    assert_eq!(
        ingest_body.result["ingested_faqs"], 1,
        "Expected 1 document to be processed."
    );

    // --- 4. Verify Database State ---
    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;
    let mut stmt = conn
        .prepare("SELECT content FROM documents WHERE source_url = 'test.pdf'")
        .await?;
    let mut rows = stmt.query(()).await?;
    let row = rows
        .next()
        .await?
        .expect("Expected to find the ingested PDF document");
    let content: String = match row.get_value(0)? {
        TursoValue::Text(s) => s,
        _ => panic!("Content was not a string"),
    };
    assert_eq!(
        content.trim(),
        expected_yaml.trim(),
        "The stored content should be the structured YAML."
    );

    // --- 5. Execute Embedding ---
    app.client
        .post(format!("{}/embed/new", app.address))
        .json(&json!({ "limit": 10 }))
        .send()
        .await?
        .error_for_status()?;

    // --- 6. Execute RAG Search ---
    let search_res = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .bearer_auth(token)
        .json(&json!({ "query": "what is the magic number?" }))
        .send()
        .await?
        .error_for_status()?;

    let search_body: ApiResponse<Value> = search_res.json().await?;
    assert_eq!(search_body.result["text"], final_rag_answer);

    // --- 7. Assert Mock Calls ---
    refinement_mock.assert();
    restructuring_mock.assert();
    metadata_extraction_mock.assert();
    query_analysis_mock.assert();
    embedding_mock.assert_hits(2); // Once for new doc, once for search query
    rag_synthesis_mock.assert();

    Ok(())
}
