//! # PDF Ingestion and RAG E2E Test
//!
//! This test verifies the entire `POST /ingest/file` workflow:
//! 1. A PDF is generated in memory with a specific, messy sentence.
//! 2. The server receives this PDF, extracts the messy text, and sends it to a mock LLM for refinement.
//! 3. The server takes the refined Markdown and sends it to a mock LLM for knowledge distillation (Q&A generation).
//! 4. The server stores the refined markdown and the Q&A pairs.
//! 5. The new Q&A pairs are embedded via a mock embedding API.
//! 6. A final RAG query (`/search/knowledge`) is made, which uses a mock LLM to synthesize an answer from the retrieved Q&A pair.
//! 7. The final answer is verified to prove the entire pipeline worked.

use anyhow::Result;
use httpmock::prelude::*;
use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, Str};
use reqwest::Client;
use serde_json::{json, Value};
use std::path::PathBuf;
use tempfile::NamedTempFile;
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};

// Include the binary's main source file to access its components.
#[path = "../src/main.rs"]
mod main;

use main::types::ApiResponse;

/// Spawns the application in the background for testing, configured with mocks.
async fn spawn_app_with_mocks(
    db_path: PathBuf,
    ai_api_url: String,
    embeddings_api_url: String,
) -> Result<String> {
    dotenvy::dotenv().ok();
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .compact()
        .try_init();

    // Set environment variables for the test instance before loading config.
    std::env::set_var("AI_API_URL", ai_api_url);
    std::env::set_var("EMBEDDINGS_API_URL", embeddings_api_url);
    std::env::set_var("EMBEDDINGS_MODEL", "mock-embedding-model");
    std::env::set_var("AI_PROVIDER", "local"); // Use the mockable provider

    let mut config = main::config::get_config().expect("Failed to load test configuration");
    config.db_url = db_path
        .to_str()
        .expect("Failed to convert temp db path to string")
        .to_string();

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{port}");

    tokio::spawn(async move {
        if let Err(e) = main::run(listener, config).await {
            eprintln!("Server error during test: {e}");
        }
    });

    sleep(Duration::from_millis(200)).await;
    Ok(address)
}

/// Generates a simple PDF with a specific messy sentence.
fn generate_test_pdf(text: &str) -> Result<Vec<u8>> {
    let mut pdf = Pdf::new();

    let catalog_id = Ref::new(1);
    let page_tree_id = Ref::new(2);
    let page_id = Ref::new(3);
    let font_id = Ref::new(4);
    let content_id = Ref::new(5);
    let font_name = Name(b"F1");

    pdf.catalog(catalog_id).pages(page_tree_id);
    pdf.pages(page_tree_id).kids([page_id]).count(1);

    let mut page = pdf.page(page_id);
    page.media_box(Rect::new(0.0, 0.0, 595.0, 842.0));
    page.parent(page_tree_id);
    page.contents(content_id);
    page.resources().fonts().pair(font_name, font_id);
    page.finish();

    pdf.type1_font(font_id).base_font(Name(b"Helvetica"));

    let mut content = Content::new();
    content.begin_text();
    content.set_font(font_name, 14.0);
    content.next_line(108.0, 734.0);
    content.show(Str(text.as_bytes()));
    content.end_text();
    pdf.stream(content_id, &content.finish());

    Ok(pdf.finish())
}

#[tokio::test]
async fn test_pdf_ingestion_and_rag_workflow() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let temp_db_file = NamedTempFile::new()?;
    let db_path = temp_db_file.path().to_path_buf();
    let mock_server = MockServer::start();
    let client = Client::new();

    // Define the magic sentence and the expected transformations
    let messy_sentence =
        "ThisIsA   messy    sentence. It needs refinement. The magic number is 3.14159.";
    let refined_markdown =
        "- This is a messy sentence.\n- It needs refinement.\n- The magic number is 3.14159.";
    let distilled_question = "What is the magic number?";
    let distilled_answer = "The magic number is 3.14159.";
    let final_rag_answer = "Based on the document, the magic number is 3.14159.";

    // --- 2. Generate Test PDF ---
    let pdf_data = generate_test_pdf(messy_sentence)?;

    // --- 3. Mock External Services ---
    let ai_api_url = mock_server.url("/v1/chat/completions");
    let embeddings_api_url = mock_server.url("/v1/embeddings");

    // A. Mock the LLM Refinement call (receives raw text, returns clean markdown)
    let refinement_mock = mock_server.mock(|when, then| {
        when.method(POST)
            .path("/v1/chat/completions")
            .body_contains("expert technical analyst"); // More robust check
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": refined_markdown}}]}),
        );
    });

    // B. Mock the Knowledge Distillation call (receives clean markdown, returns Q&A)
    let distillation_mock = mock_server.mock(|when, then| {
        when.method(POST)
            .path("/v1/chat/completions")
            .body_contains("reconciliation agent"); // Check for the distillation prompt
        then.status(200)
            .json_body(json!({"choices": [{"message": {"role": "assistant", "content": json!({
                "faqs": [{ "question": distilled_question, "answer": distilled_answer, "is_explicit": false }],
                "content_chunks": []
            }).to_string()}}]}));
    });

    // C. Mock the Embedding API call
    let embedding_mock = mock_server.mock(|when, then| {
        when.method(POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.1, 0.2, 0.3] }] }));
    });

    // D. Mock the final RAG synthesis call (receives context, returns final answer)
    let rag_synthesis_mock = mock_server.mock(|when, then| {
        when.method(POST)
            .path("/v1/chat/completions")
            .body_contains("strict, factual AI"); // Check for the RAG prompt
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_rag_answer}}]}),
        );
    });

    // --- 4. Spawn App ---
    let app_address = spawn_app_with_mocks(db_path, ai_api_url, embeddings_api_url).await?;

    // --- 5. Execute Ingestion ---
    let form = reqwest::multipart::Form::new()
        .part(
            "file",
            reqwest::multipart::Part::bytes(pdf_data).file_name("test.pdf"),
        )
        .part("extractor", reqwest::multipart::Part::text("local"));

    let ingest_res = client
        .post(format!("{app_address}/ingest/file"))
        .multipart(form)
        .send()
        .await?
        .error_for_status()?;

    let ingest_body: ApiResponse<Value> = ingest_res.json().await?;
    assert_eq!(ingest_body.result["ingested_faqs"], 1);

    // --- 6. Execute Embedding ---
    client
        .post(format!("{app_address}/embed/faqs/new"))
        .json(&json!({ "limit": 10 }))
        .send()
        .await?
        .error_for_status()?;

    // --- 7. Execute RAG Search and Verify ---
    let search_res = client
        .post(format!("{app_address}/search/knowledge"))
        .json(&json!({ "query": "what is the magic number?" }))
        .send()
        .await?
        .error_for_status()?;

    let search_body: ApiResponse<Value> = search_res.json().await?;
    assert_eq!(search_body.result["text"], final_rag_answer);

    // --- 8. Assert Mock Calls ---
    refinement_mock.assert();
    distillation_mock.assert();
    // Embedding is called twice: once for the new FAQ, once for the search query.
    embedding_mock.assert_hits(2);
    rag_synthesis_mock.assert();

    Ok(())
}
