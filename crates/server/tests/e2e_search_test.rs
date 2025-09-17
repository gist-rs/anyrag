//! # End-to-End Search Tests
//!
//! This file contains E2E tests for the `/search/hybrid` endpoint, verifying
//! both the LLM-based re-ranking and RRF (Reciprocal Rank Fusion) modes.

mod common;

use anyhow::Result;
use anyrag_server::types::ApiResponse;
use common::{generate_jwt, TestApp};
use httpmock::Method;
use serde_json::{json, Value};

#[tokio::test]
async fn test_hybrid_search_llm_and_rrf_modes() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn("test_hybrid_search_llm_and_rrf_modes").await?;
    let token = generate_jwt("search-test-user@example.com")?;

    // --- 2. Mock External Services ---

    // A. Mock the RSS feed to provide two distinct articles.
    let rss_content = r#"
<rss version="2.0">
<channel>
  <title>Test Feed</title>
  <link>http://mock.com/rss</link>
  <description>A test feed for AnyRAG.</description>
  <item>
    <title>Learning Rust</title>
    <link>http://mock.com/rust</link>
    <description>Rust is a systems programming language.</description>
  </item>
  <item>
    <title>Learning Go</title>
    <link>http://mock.com/go</link>
    <description>Go is another systems language.</description>
  </item>
</channel>
</rss>
"#;
    let rss_mock = app.mock_server.mock(|when, then| {
        when.method(Method::GET).path("/rss");
        then.status(200).body(rss_content);
    });

    // B. Mock the Embedding API.
    // We need two separate mocks because the application makes two distinct calls:
    // one to embed the documents during ingestion, and another to embed the user's query.

    // This mock handles the batch embedding of the two documents from the RSS feed.
    let doc_embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/test_hybrid_search_llm_and_rrf_modes/v1/embeddings")
            .body_contains("Learning Rust"); // Differentiate it from the query embedding call
        then.status(200).json_body(json!({
            "data": [
                { "embedding": [0.1, 0.2, 0.3] }, // Embedding for "Learning Rust"
                { "embedding": [0.4, 0.5, 0.6] }  // Embedding for "Learning Go"
            ]
        }));
    });

    // This mock handles embedding the user's search query.
    let query_embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/test_hybrid_search_llm_and_rrf_modes/v1/embeddings")
            .body_contains("What is golang?"); // The actual search query
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.3, 0.4, 0.5] }] }));
    });

    // C. Mock the LLM Re-rank call.
    // The AI should respond with the "Go" article first, as it's more relevant to the query "golang".
    let llm_rerank_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/test_hybrid_search_llm_and_rrf_modes/v1/chat/completions")
            .body_contains("expert search result re-ranker"); // Match the llm_rerank system prompt
        then.status(200).json_body(json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": json!(["http://mock.com/go", "http://mock.com/rust"]).to_string()
                }
            }]
        }));
    });

    // --- 3. Act: Ingest and Embed ---
    // Ingest the 2 articles from the mocked RSS feed.
    app.client
        .post(format!("{}/ingest/rss", app.address))
        .bearer_auth(token.clone())
        .json(&json!({ "url": app.mock_server.url("/rss") }))
        .send()
        .await?
        .error_for_status()?;

    // Embed the 2 new documents. This will trigger the `doc_embedding_mock`.
    app.client
        .post(format!("{}/embed/new", app.address))
        .json(&json!({ "limit": 2 }))
        .send()
        .await?
        .error_for_status()?;

    // --- 4. Act: Test LLM Re-rank Mode ---
    let llm_search_res = app
        .client
        .post(format!("{}/search/hybrid", app.address))
        .bearer_auth(token.clone())
        .json(&json!({
            "query": "What is golang?",
            // THIS IS THE FIX: The enum variant is `LlmReRank`, which serializes to `llm_re_rank`.
            "mode": "llm_re_rank"
        }))
        .send()
        .await?
        .error_for_status()?;

    let llm_body: ApiResponse<Value> = llm_search_res.json().await?;
    let llm_results = llm_body.result.as_array().unwrap();

    // Assert: Check that LLM re-ranking returned "Go" first.
    assert_eq!(llm_results.len(), 2);
    assert_eq!(llm_results[0]["title"], "Learning Go");
    assert_eq!(llm_results[1]["title"], "Learning Rust");

    // --- 5. Act: Test RRF Mode ---
    let rrf_search_res = app
        .client
        .post(format!("{}/search/hybrid", app.address))
        .bearer_auth(token.clone())
        .json(&json!({
            "query": "What is golang?",
            "mode": "rrf"
        }))
        .send()
        .await?
        .error_for_status()?;

    let rrf_body: ApiResponse<Value> = rrf_search_res.json().await?;
    let rrf_results = rrf_body.result.as_array().unwrap();

    // Assert: RRF might produce a different order, but it should still return both results.
    assert_eq!(rrf_results.len(), 2);

    // --- 6. Assert Mocks ---
    rss_mock.assert();
    doc_embedding_mock.assert();
    query_embedding_mock.assert_hits(2); // Called once for each search mode
    llm_rerank_mock.assert();

    Ok(())
}
