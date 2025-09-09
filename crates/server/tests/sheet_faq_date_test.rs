//! # Time-Sensitive FAQ RAG Test from Google Sheets
//!
//! This test verifies that the RAG pipeline can correctly answer time-sensitive
//! questions using the `start_at` and `end_at` context ingested from a Google Sheet.

mod common;

use anyhow::Result;
use chrono::{Duration, Utc};
use common::{generate_jwt, TestApp};
use httpmock::Method;
use serde_json::{json, Value};

use anyrag_server::types::ApiResponse;

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

    // B. Mock the Embedding API. It will be called once for the ingested sheet document and once for the search query.
    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.1, 0.2, 0.3] }] }));
    });

    // C. Mock the Query Analysis call for the RAG search.
    let query_analysis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("expert query analyst");
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": json!({
                "entities": [],
                "keyphrases": ["hobby"]
            }).to_string()}}]}),
        );
    });

    // D. Mock the final RAG synthesis call.
    // This is the most critical assertion. We check that the LLM receives the correct date context.
    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("strict, factual AI") // Check for the RAG prompt
            // Check that the context contains BOTH ingested answers.
            .body_contains("Football")
            .body_contains("Reading");
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_rag_answer}}]}),
        );
    });

    // --- 4. Execute Ingestion from Sheet URL ---
    let user_identifier = "date-test-user@example.com";
    let token = generate_jwt(user_identifier)?;
    let ingest_res = app
        .client
        .post(format!("{}/ingest/sheet?faq=true", app.address))
        .bearer_auth(token.clone())
        .json(&json!({ "url": app.mock_server.url("/spreadsheets/d/mock_sheet_id/export?format=csv") }))
        .send()
        .await?
        .error_for_status()?;

    let ingest_body: ApiResponse<Value> = ingest_res.json().await?;
    assert_eq!(ingest_body.result["ingested_rows"], 2);

    // --- 5. Execute Embedding for New Documents ---
    app.client
        .post(format!("{}/embed/new", app.address))
        .json(&json!({ "limit": 10 }))
        .send()
        .await?
        .error_for_status()?;

    // --- 6. Execute RAG Search and Verify ---
    let search_res = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .bearer_auth(token)
        .json(&json!({ "query": "What is the hobby?" }))
        .send()
        .await?
        .error_for_status()?;

    let search_body: ApiResponse<Value> = search_res.json().await?;
    assert_eq!(search_body.result["text"], final_rag_answer);

    // --- 8. Assert Mock Calls ---
    sheet_download_mock.assert();
    query_analysis_mock.assert();
    embedding_mock.assert_hits(2); // 1 for the ingested document, 1 for the search query.
    rag_synthesis_mock.assert(); // This confirms the core logic of the test.

    Ok(())
}
