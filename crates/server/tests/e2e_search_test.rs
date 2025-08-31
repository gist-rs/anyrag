//! # End-to-End Hybrid Search Workflow Tests
//!
//! This file tests the hybrid search endpoint, verifying both the default
//! LLM-based re-ranking and the optional RRF re-ranking modes. It uses mocks
//! for all external services to ensure deterministic and reliable results.

mod common;

use anyhow::Result;
use common::TestApp;
use httpmock::Method;
use serde_json::{json, Value};
use tracing::info;

use common::main::types::ApiResponse;

#[tokio::test]
async fn test_hybrid_search_llm_and_rrf_modes() -> Result<()> {
    // --- 1. Arrange ---
    let app = TestApp::spawn().await?;

    // --- 2. Mock External Services ---

    // A. Mock the RSS feed with two distinct articles.
    // The descriptions are crafted to ensure keyword search finds both.
    let mock_rss_content = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <rss version="2.0">
        <channel>
            <title>Mock Feed</title>
            <link>http://m.com</link>
            <description>A mock feed for testing.</description>
            <item>
                <title>The Rise of Qwen3</title>
                <link>http://m.com/qwen3</link>
                <description>An article about a database and large language models.</description>
                <pubDate>Mon, 01 Jan 2024 12:00:00 GMT</pubDate>
            </item>
            <item>
                <title>PostgreSQL Performance</title>
                <link>http://m.com/postgres</link>
                <description>An article about database tuning.</description>
                <pubDate>Tue, 02 Jan 2024 12:00:00 GMT</pubDate>
            </item>
        </channel>
        </rss>
    "#;
    app.mock_server.mock(|when, then| {
        when.method(Method::GET).path("/rss");
        then.status(200).body(mock_rss_content);
    });

    // B. Mock the Embeddings API to return a generic, non-null vector for any input.
    // This ensures vector search always returns all documents, guaranteeing candidates for re-ranking.
    let embeddings_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.1, 0.2, 0.3, 0.0] }] }));
    });

    // C. Mock the LLM Re-ranking API.
    // It will be queried with "database" and is programmed to prefer the PostgreSQL article.
    let rerank_response_content = json!([
        "http://m.com/postgres", // The LLM's top choice
        "http://m.com/qwen3"
    ])
    .to_string();

    let reranker_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("expert search result re-ranker"); // Differentiate from other AI calls
        then.status(200).json_body(json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": rerank_response_content
                }
            }]
        }));
    });

    // --- 3. Spawn App, Ingest, and Embed ---
    app.client
        .post(format!("{}/ingest", app.address))
        .json(&json!({ "url": app.mock_server.url("/rss") }))
        .send()
        .await?
        .error_for_status()?;

    app.client
        .post(format!("{}/embed/new", app.address))
        .json(&json!({ "limit": 2 }))
        .send()
        .await?
        .error_for_status()?;
    info!("-> Ingest & Embed Complete.");

    // --- 4. Test LLM Re-ranking (Default Mode) ---
    info!("\n--- Testing Hybrid Search with LLM Re-ranking (Default) ---");
    // This query ensures both articles are candidates, letting us test the re-ranker.
    let llm_res = app
        .client
        .post(format!("{}/search/hybrid", app.address))
        .json(&json!({ "query": "database" })) // Use a query that will match via keyword
        .send()
        .await?
        .error_for_status()?;

    let llm_response: ApiResponse<Vec<Value>> = llm_res.json().await?;
    let llm_results = llm_response.result;
    info!("LLM Re-ranked Results: {:?}", llm_results);

    assert_eq!(
        llm_results.len(),
        2,
        "Expected LLM to re-rank two candidates."
    );
    assert_eq!(
        llm_results[0]["title"], "PostgreSQL Performance",
        "LLM re-ranking did not place the PostgreSQL article first."
    );
    assert_eq!(
        llm_results[1]["title"], "The Rise of Qwen3",
        "LLM re-ranking did not place the Qwen3 article second."
    );

    // --- 5. Test RRF Re-ranking (Optional Mode) ---
    info!("\n--- Testing Hybrid Search with RRF Re-ranking (Optional) ---");
    let rrf_res = app
        .client
        .post(format!("{}/search/hybrid", app.address))
        .json(&json!({
            "query": "PostgreSQL", // This query strongly favors the keyword match
            "mode": "rrf"
        }))
        .send()
        .await?
        .error_for_status()?;

    let rrf_response: ApiResponse<Vec<Value>> = rrf_res.json().await?;
    let rrf_results = rrf_response.result;
    info!("RRF Results: {:?}", rrf_results);
    // Since the vector search is generic and returns both, and the keyword search returns one,
    // the RRF should correctly combine and boost the keyword match to the top.
    assert_eq!(rrf_results.len(), 2, "Expected RRF to rank two candidates.");
    assert_eq!(
        rrf_results[0]["title"], "PostgreSQL Performance",
        "RRF did not prioritize the strong keyword match for PostgreSQL."
    );

    // --- 6. Assert Mock Calls ---
    // Embeddings: 2 for ingest, 1 for LLM search, 1 for RRF search = 4
    embeddings_mock.assert_hits(4);
    // Re-ranker: Only called once for the default (LLM) mode search
    reranker_mock.assert_hits(1);

    Ok(())
}
