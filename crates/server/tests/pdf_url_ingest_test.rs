//! # PDF URL Ingestion and RAG E2E Test (YAML Pipeline)
//!
//! This test verifies the entire `POST /ingest/pdf` workflow using a URL,
//! based on the new, structured YAML pipeline. It ensures that:
//! 1. A mock server serves a PDF, which is downloaded by the app.
//! 2. The PDF's raw text is extracted, refined, and then restructured into YAML via mock LLM calls.
//! 3. The final YAML content is stored as a single document in the database.
//! 4. The new document is embedded via a mock embedding API.
//! 5. A final RAG query (`/search/knowledge`) correctly chunks the YAML,
//!    retrieves the relevant context, and synthesizes the correct answer.

mod common;

use anyhow::Result;
use anyrag_server::{config, state::build_app_state, types::ApiResponse};
use common::{generate_jwt, pdf_helper::generate_test_pdf, TestApp};
use httpmock::{Method, MockServer};
use serde_json::{json, Value};
use std::fs::File;
use std::io::Write;
use tempfile::{tempdir, NamedTempFile};
use turso::{params, Builder, Value as TursoValue};

#[tokio::test]
async fn test_pdf_url_ingestion_and_rag_workflow_yaml() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let mock_server = MockServer::start();
    let db_file = NamedTempFile::new()?;
    let db_path = db_file.path();

    let config_dir = tempdir()?;
    let config_path = config_dir.path().join("config.yml");

    let config_content = format!(
        r#"
port: 0
db_url: "{}"
embedding:
  api_url: "{}"
  model_name: "mock-embedding-model"
providers:
  local_default:
    provider: "local"
    api_url: "{}"
    api_key: null
    model_name: "mock-chat-model"
tasks:
  knowledge_distillation:
    provider: "local_default"
  knowledge_metadata_extraction:
    provider: "local_default"
  query_analysis:
    provider: "local_default"
  rag_synthesis:
    provider: "local_default"
"#,
        db_path.to_str().unwrap(),
        mock_server.url("/v1/embeddings"),
        mock_server.url("/v1/chat/completions")
    );

    let mut file = File::create(&config_path)?;
    file.write_all(config_content.as_bytes())?;

    let config = config::get_config(Some(config_path.to_str().unwrap()))?;
    let app_state = build_app_state(config).await?;
    let app = TestApp::spawn_with_state(app_state, mock_server).await?;

    let messy_sentence = "The magic number from the URL is 3.14159.";
    let refined_markdown = "- The magic number from the URL is 3.14159.";
    let expected_yaml = r#"
sections:
  - title: "General Information"
    faqs:
      - question: "What is the magic number?"
        answer: "The magic number from the URL is 3.14159."
"#;
    let final_rag_answer = "The magic number from the URL is 3.14159.";

    let pdf_data = generate_test_pdf(messy_sentence)?;

    // --- 2. Mock External Services ---
    // A. Mock the server that will provide the PDF file to the application.
    let pdf_serve_mock = app.mock_server.mock(|when, then| {
        when.method(Method::GET).path("/test.pdf");
        then.status(200)
            .header("Content-Type", "application/pdf")
            .body(&pdf_data);
    });

    // B. Mock LLM Refinement (raw text -> clean markdown). This was missing.
    let refinement_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("You are an expert technical analyst"); // Matches PDF_REFINEMENT_SYSTEM_PROMPT
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": refined_markdown}}]}),
        );
    });

    // C. Mock LLM Restructuring (clean markdown -> structured YAML)
    let restructuring_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("expert document analyst and editor"); // Matches KNOWLEDGE_RESTRUCTURING_SYSTEM_PROMPT
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": expected_yaml}}]}),
        );
    });

    // D. Mock Metadata Extraction (structured YAML -> metadata JSON)
    let metadata_extraction_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("extract Category, Keyphrases, and Entities");
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": json!([
                {"type": "KEYPHRASE", "subtype": "CONCEPT", "value": "magic number"}
            ]).to_string()}}]}),
        );
    });

    // E. Mock Embedding API
    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.1, 0.2, 0.3] }] }));
    });

    // F. Mock RAG Query Analysis
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

    // G. Mock final RAG synthesis call
    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("strict, factual AI")
            .body_contains("## General Information");
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_rag_answer}}]}),
        );
    });

    // --- 3. Execute Ingestion ---
    let user_identifier = "pdf-url-ingest-user@example.com";
    let token = generate_jwt(user_identifier)?;

    let form = reqwest::multipart::Form::new()
        .part(
            "url",
            reqwest::multipart::Part::text(app.mock_server.url("/test.pdf")),
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
    assert_eq!(ingest_body.result["ingested_faqs"], 1);

    // --- 4. Verify Database State ---
    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;
    let mut stmt = conn
        .prepare("SELECT content FROM documents WHERE source_url = ?")
        .await?;

    // The handler logic takes the last part of the URL path as the source identifier.
    let pdf_filename = "test.pdf";

    let mut rows = stmt.query(params![pdf_filename]).await?;
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
    pdf_serve_mock.assert();
    refinement_mock.assert();
    restructuring_mock.assert();
    metadata_extraction_mock.assert();
    query_analysis_mock.assert();
    embedding_mock.assert_hits(2); // Once for new doc, once for search query
    rag_synthesis_mock.assert();

    Ok(())
}
