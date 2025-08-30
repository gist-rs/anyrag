//! # Time-Sensitive FAQ RAG Test from Google Sheets
//!
//! This test verifies that the RAG pipeline can correctly answer time-sensitive
//! questions using the `start_at` and `end_at` context ingested from a Google Sheet.

mod common;

use anyhow::Result;
use chrono::{Duration, Utc};
use common::TestApp;
use httpmock::Method;
use serde_json::{json, Value};

use common::main::types::ApiResponse;

#[tokio::test]
async fn test_sheet_faq_date_sensitive_rag_workflow() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn().await?;

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
    // A. Mock the Google Sheet CSV download.
    let sheet_download_mock = app.mock_server.mock(|when, then| {
        when.method(Method::GET)
            .path_contains("/export") // A bit more flexible than the full path
            .query_param("format", "csv");
        then.status(200).body(csv_data);
    });

    // B. Mock the Embedding API. It will be called for the 2 new FAQs and the search query.
    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.1, 0.2, 0.3] }] }));
    });

    // C. Mock the final RAG synthesis call.
    // This is the most critical assertion. We check that the LLM receives the correct date context.
    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
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

    // --- 4. Execute Ingestion from Sheet URL ---
    let ingest_res = app
        .client
        .post(format!("{}/ingest/sheet_faq", app.address))
        .json(&json!({ "url": app.mock_server.url("/spreadsheets/d/mock_sheet_id/export?format=csv") }))
        .send()
        .await?
        .error_for_status()?;

    let ingest_body: ApiResponse<Value> = ingest_res.json().await?;
    assert_eq!(ingest_body.result["ingested_faqs"], 2);

    // --- 5. Execute Embedding for New FAQs ---
    app.client
        .post(format!("{}/embed/faqs/new", app.address))
        .json(&json!({ "limit": 10 }))
        .send()
        .await?
        .error_for_status()?;

    // --- 6. Execute RAG Search and Verify ---
    let search_res = app
        .client
        .post(format!("{}/search/knowledge", app.address))
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
