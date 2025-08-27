//! # Time-Sensitive FAQ RAG Test from Google Sheets
//!
//! This test verifies that the RAG pipeline can correctly answer time-sensitive
//! questions using the `start_at` and `end_at` context ingested from a Google Sheet.

use anyhow::Result;
use chrono::{Duration, Utc};
use httpmock::prelude::*;
use reqwest::Client;
use serde_json::{json, Value};
use std::path::PathBuf;
use tempfile::NamedTempFile;
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration as TokioDuration};

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

    sleep(TokioDuration::from_millis(200)).await;
    Ok(address)
}

#[tokio::test]
async fn test_sheet_faq_date_sensitive_rag_workflow() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let temp_db_file = NamedTempFile::new()?;
    let db_path = temp_db_file.path().to_path_buf();
    let mock_server = MockServer::start();
    let client = Client::new();

    // --- 2. Define Test Data with Dynamic Dates ---
    let today = Utc::now();
    let current_hobby_start = (today - Duration::days(1)).format("%Y-%m-%d").to_string();
    let current_hobby_end = (today + Duration::days(1)).format("%Y-%m-%d").to_string();

    let csv_data = format!(
        "Questions,Answers,start_at,end_at\n\
         Hobby?,Reading,2020-01-01,2021-01-01\n\
         Hobby?,Football,{current_hobby_start},{current_hobby_end}",
    );

    let final_rag_answer = "The current hobby is Football.";

    // --- 3. Mock External Services ---
    let ai_api_url = mock_server.url("/v1/chat/completions");
    let embeddings_api_url = mock_server.url("/v1/embeddings");

    // A. Mock the Google Sheet CSV download.
    let sheet_download_mock = mock_server.mock(|when, then| {
        when.method(GET)
            .path_contains("/export") // A bit more flexible than the full path
            .query_param("format", "csv");
        then.status(200).body(csv_data);
    });

    // B. Mock the Embedding API. It will be called for the 2 new FAQs and the search query.
    let embedding_mock = mock_server.mock(|when, then| {
        when.method(POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.1, 0.2, 0.3] }] }));
    });

    // C. Mock the final RAG synthesis call.
    // This is the most critical assertion. We check that the LLM receives the correct date context.
    let rag_synthesis_mock = mock_server.mock(|when, then| {
        when.method(POST)
            .path("/v1/chat/completions")
            .body_contains("strict, factual AI") // Check for the RAG prompt
            // Check that the context contains BOTH ingested answers with their date ranges.
            .body_contains(format!(
                "Football (effective from {current_hobby_start} to {current_hobby_end})",
            ))
            .body_contains("Reading (effective from 2020-01-01 to 2021-01-01)");
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_rag_answer}}]}),
        );
    });

    // --- 4. Spawn App ---
    let app_address = spawn_app_with_mocks(db_path, ai_api_url, embeddings_api_url).await?;

    // --- 5. Execute Ingestion from Sheet URL ---
    let ingest_res = client
        .post(format!("{app_address}/ingest/sheet_faq"))
        .json(&json!({ "url": mock_server.url("/spreadsheets/d/mock_sheet_id/export?format=csv") }))
        .send()
        .await?
        .error_for_status()?;

    let ingest_body: ApiResponse<Value> = ingest_res.json().await?;
    assert_eq!(ingest_body.result["ingested_faqs"], 2);

    // --- 6. Execute Embedding for New FAQs ---
    client
        .post(format!("{app_address}/embed/faqs/new"))
        .json(&json!({ "limit": 10 }))
        .send()
        .await?
        .error_for_status()?;

    // --- 7. Execute RAG Search and Verify ---
    let search_res = client
        .post(format!("{app_address}/search/knowledge"))
        .json(&json!({ "query": "What is the hobby?" }))
        .send()
        .await?
        .error_for_status()?;

    let search_body: ApiResponse<Value> = search_res.json().await?;
    assert_eq!(search_body.result["text"], final_rag_answer);

    // --- 8. Assert Mock Calls ---
    sheet_download_mock.assert();
    embedding_mock.assert_hits(3); // 2 for ingest, 1 for search
    rag_synthesis_mock.assert(); // This confirms the core logic of the test.

    Ok(())
}
