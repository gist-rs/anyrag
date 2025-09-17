//! # Unified Ingestion Pipeline E2E Test
//!
//! This file contains a comprehensive integration test for the new, unified ingestion
//! pipeline. It replaces the old tests for the deprecated `?faq=true` and `?faq=false`
//! parameters. This single test verifies the entire end-to-end workflow: from PDF
//! ingestion to structured YAML storage, embedding, and a final RAG query.

mod common;

use anyhow::Result;
use anyrag_server::types::ApiResponse;
use common::{generate_jwt, pdf_helper::generate_test_pdf, TestApp};
use httpmock::Method;
use serde_json::{json, Value};
use turso::{params, Builder, Value as TursoValue};

#[tokio::test]
async fn test_unified_pdf_ingestion_and_rag_workflow() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn().await?;
    let token = generate_jwt("unified-ingest-user@example.com")?;

    let pdf_content = "The magic word is AnyRAG. It is a powerful framework.";
    let pdf_data = generate_test_pdf(pdf_content)?;

    // The intermediate step where raw text is cleaned into Markdown.
    let refined_markdown = "- The magic word is AnyRAG.\n- It is a powerful framework.";

    // The final structured data that should be stored in the database.
    let expected_yaml = r#"
sections:
  - title: "General Information"
    faqs:
      - question: "What is the magic word?"
        answer: "The magic word is AnyRAG."
      - question: "What is AnyRAG?"
        answer: "It is a powerful framework."
"#;
    let final_rag_answer = "AnyRAG is a powerful framework.";

    // --- 2. Mock External Services ---

    // A. Mock the LLM Refinement call (raw text -> clean markdown).
    let refinement_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            // Matcher for PDF_REFINEMENT_SYSTEM_PROMPT
            .body_contains("expert technical analyst");
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": refined_markdown}}]}),
        );
    });

    // B. Mock the LLM Restructuring call (clean markdown -> structured YAML)
    let restructuring_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            // Matcher for KNOWLEDGE_RESTRUCTURING_SYSTEM_PROMPT
            .body_contains("expert document analyst and editor");
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": expected_yaml}}]}),
        );
    });

    // C. Mock the Metadata Extraction call
    let metadata_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("extract Category, Keyphrases, and Entities");
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": json!([
                {"type": "KEYPHRASE", "subtype": "CONCEPT", "value": "magic word"},
                {"type": "ENTITY", "subtype": "PRODUCT", "value": "AnyRAG"}
            ]).to_string()}}]}),
        );
    });

    // D. Mock the Embedding API (for new doc and for search query)
    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.1, 0.2, 0.3] }] }));
    });

    // E. Mock the RAG Query Analysis
    let query_analysis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("expert query analyst");
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": json!({
                "entities": ["AnyRAG"], "keyphrases": []
            }).to_string()}}]}),
        );
    });

    // F. Mock the final RAG Synthesis call
    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("strict, factual AI") // Updated to match the new default prompt
            .body_contains("## General Information");
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_rag_answer}}]}),
        );
    });

    // --- 3. Act: Ingest the PDF ---
    let form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(pdf_data).file_name("test.pdf"),
    );

    let ingest_response = app
        .client
        .post(format!("{}/ingest/pdf", app.address))
        .bearer_auth(token.clone())
        .multipart(form)
        .send()
        .await?
        .error_for_status()?;

    // Assert API Response for ingestion
    let ingest_body: ApiResponse<Value> = ingest_response.json().await?;
    assert_eq!(ingest_body.result["ingested_documents"], 1);

    // --- 4. Assert Database State ---
    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;
    let mut stmt = conn
        .prepare("SELECT content FROM documents WHERE source_url = ?")
        .await?;
    let mut rows = stmt.query(params!["test.pdf"]).await?;
    let row = rows.next().await?.expect("Document not found in DB");
    let stored_content: String = match row.get_value(0)? {
        TursoValue::Text(s) => s,
        _ => panic!("Content was not a string"),
    };
    assert_eq!(
        stored_content.trim(),
        expected_yaml.trim(),
        "Stored content should be the structured YAML"
    );

    // --- 5. Act: Embed the new document ---
    app.client
        .post(format!("{}/embed/new", app.address))
        .json(&json!({ "limit": 10 }))
        .send()
        .await?
        .error_for_status()?;

    // --- 6. Act: Perform a RAG search ---
    let search_response = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .bearer_auth(token)
        .json(&json!({ "query": "What is AnyRAG?" }))
        .send()
        .await?
        .error_for_status()?;

    // Assert final RAG response
    let search_body: ApiResponse<Value> = search_response.json().await?;
    assert_eq!(search_body.result["text"], final_rag_answer);

    // --- 7. Assert All Mocks Were Called ---
    refinement_mock.assert();
    restructuring_mock.assert();
    metadata_mock.assert();
    embedding_mock.assert_hits(2); // Once for ingestion, once for search
    query_analysis_mock.assert();
    rag_synthesis_mock.assert();

    Ok(())
}
