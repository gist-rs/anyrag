//! # End-to-End Hybrid Search Workflow Tests
//!
//! This file tests the hybrid search endpoint, verifying both the default
//! LLM-based re-ranking and the optional RRF re-ranking modes. It uses mocks
//! for all external services to ensure deterministic and reliable results.

use anyhow::Result;
use httpmock::prelude::*;
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

/// Spawns the application in the background for testing, configured with a temporary DB
/// and mock URLs for all external services.
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
    std::env::set_var("AI_PROVIDER", "local"); // Ensure we use the mockable provider

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

#[tokio::test]
async fn test_hybrid_search_llm_and_rrf_modes() -> Result<()> {
    // --- 1. Arrange ---
    let temp_db_file = NamedTempFile::new()?;
    let db_path = temp_db_file.path().to_path_buf();
    let server = MockServer::start();
    let client = Client::new();

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
    server.mock(|when, then| {
        when.method(GET).path("/rss");
        then.status(200).body(mock_rss_content);
    });

    // B. Mock the Embeddings API to return a generic, non-null vector for any input.
    // This ensures vector search always returns all documents, guaranteeing candidates for re-ranking.
    let embeddings_mock = server.mock(|when, then| {
        when.method(POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.1, 0.2, 0.3] }] }));
    });

    // C. Mock the LLM Re-ranking API.
    // It will be queried with "database" and is programmed to prefer the PostgreSQL article.
    let rerank_response_content = json!([
        "http://m.com/postgres", // The LLM's top choice
        "http://m.com/qwen3"
    ])
    .to_string();

    let reranker_mock = server.mock(|when, then| {
        when.method(POST).path("/v1/chat/completions");
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
    let app_address = spawn_app_with_mocks(
        db_path,
        server.url("/v1/chat/completions"),
        server.url("/v1/embeddings"),
    )
    .await?;

    client
        .post(format!("{app_address}/ingest"))
        .json(&json!({ "url": server.url("/rss") }))
        .send()
        .await?
        .error_for_status()?;

    client
        .post(format!("{app_address}/embed/new"))
        .json(&json!({ "limit": 2 }))
        .send()
        .await?
        .error_for_status()?;
    println!("-> Ingest & Embed Complete.");

    // --- 4. Test LLM Re-ranking (Default Mode) ---
    println!("\n--- Testing Hybrid Search with LLM Re-ranking (Default) ---");
    // This query ensures both articles are candidates, letting us test the re-ranker.
    let llm_res = client
        .post(format!("{app_address}/search/hybrid"))
        .json(&json!({ "query": "database" })) // Use a query that will match via keyword
        .send()
        .await?
        .error_for_status()?;

    let llm_response: ApiResponse<Vec<Value>> = llm_res.json().await?;
    let llm_results = llm_response.result;
    println!("LLM Re-ranked Results: {llm_results:?}");

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
    println!("\n--- Testing Hybrid Search with RRF Re-ranking (Optional) ---");
    let rrf_res = client
        .post(format!("{app_address}/search/hybrid"))
        .json(&json!({
            "query": "PostgreSQL", // This query strongly favors the keyword match
            "mode": "rrf"
        }))
        .send()
        .await?
        .error_for_status()?;

    let rrf_response: ApiResponse<Vec<Value>> = rrf_res.json().await?;
    let rrf_results = rrf_response.result;
    println!("RRF Results: {rrf_results:?}");
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
