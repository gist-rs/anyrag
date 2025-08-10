//! # End-to-End Search Workflow Tests
//!
//! This file contains comprehensive integration tests that simulate the full
//! user workflow: ingesting articles, embedding them, and then using the
//! various search endpoints to find them. This version uses a mock embeddings
//! server to ensure the tests are deterministic and isolated from external services.

use anyhow::Result;
use httpmock::prelude::*;
use regex::Regex;
use reqwest::Client;
use serde_json::json;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};

// Include the binary's main source file to access its components.
#[path = "../src/main.rs"]
mod main;

/// Spawns the application in the background for testing, configured with a temporary DB and a specific embeddings URL.
async fn spawn_app_with_mocks(db_path: PathBuf, embeddings_api_url: String) -> Result<String> {
    dotenvy::dotenv().ok();
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .compact()
        .try_init();

    // Set environment variables for the test instance before loading config.
    std::env::set_var("EMBEDDINGS_API_URL", embeddings_api_url);
    std::env::set_var("EMBEDDINGS_MODEL", "mock-embedding-model");

    // Load configuration, which will now pick up the mock env vars.
    // Then, override the db_url to use our temporary database.
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

    sleep(Duration::from_millis(200)).await; // Give server a moment to start

    Ok(address)
}

#[tokio::test]
async fn test_e2e_all_search_endpoints_with_mock_embeddings() -> Result<()> {
    // --- 1. Arrange ---
    let temp_db_file = NamedTempFile::new()?;
    let db_path = temp_db_file.path().to_path_buf();

    // --- Mock Servers Setup ---
    let server = MockServer::start();

    // A. Mock the RSS feed server.
    let mock_rss_content = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <rss version="2.0">
        <channel>
            <title>Mock Feed</title>
            <link>http://mock.com</link>
            <description>A mock feed for testing search endpoints.</description>
            <item>
                <title>A Deep Dive into PostgreSQL Performance</title>
                <link>http://mock.com/postgres</link>
                <description>Optimizing queries and indexes in your database.</description>
                <pubDate>Mon, 01 Jan 2024 12:00:00 GMT</pubDate>
            </item>
            <item>
                <title>The Rise of Qwen3</title>
                <link>http://mock.com/qwen3</link>
                <description>Exploring the new features of the Qwen3 large language model.</description>
                <pubDate>Tue, 02 Jan 2024 12:00:00 GMT</pubDate>
            </item>
            <item>
                <title>Building Web Apps with Rust</title>
                <link>http://mock.com/rust-web</link>
                <description>A tutorial on creating interactive sites using Axum.</description>
                <pubDate>Wed, 03 Jan 2024 12:00:00 GMT</pubDate>
            </item>
        </channel>
        </rss>
    "#;
    server.mock(|when, then| {
        when.method(GET).path("/rss");
        then.status(200)
            .header("content-type", "application/rss+xml")
            .body(mock_rss_content);
    });
    let rss_feed_url = server.url("/rss");

    // B. Mock the Embeddings API server with predictable vectors.
    let embeddings_url = server.url("/v1/embeddings");
    let postgres_vec = vec![1.0, 0.0, 0.0];
    let qwen3_vec = vec![0.0, 1.0, 0.0];
    let rust_vec = vec![0.0, 0.0, 1.0];

    // Create Regex objects to pass to the mock setup.
    let postgres_regex = Regex::new("(?i)postgres").unwrap();
    let qwen3_regex = Regex::new("(?i)qwen3").unwrap();
    let rust_regex = Regex::new("(?i)rust").unwrap();

    let postgres_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v1/embeddings")
            .body_matches(postgres_regex);
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "data": [{ "embedding": postgres_vec }] }));
    });

    let qwen3_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v1/embeddings")
            .body_matches(qwen3_regex);
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "data": [{ "embedding": qwen3_vec }] }));
    });

    let rust_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v1/embeddings")
            .body_matches(rust_regex);
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "data": [{ "embedding": rust_vec }] }));
    });

    // --- Spawn App ---
    let app_address = spawn_app_with_mocks(db_path.clone(), embeddings_url).await?;
    let client = Client::new();

    // --- 2. Ingest & Embed ---
    println!("\n--- Step 1: Ingesting and Embedding Articles ---");
    client
        .post(format!("{app_address}/ingest"))
        .json(&json!({ "url": rss_feed_url }))
        .send()
        .await?
        .error_for_status()?;
    client
        .post(format!("{app_address}/embed/new"))
        .json(&json!({ "limit": 3 })) // Embed all 3 articles
        .send()
        .await?
        .error_for_status()?;
    println!("-> Ingest & Embed Complete.");

    // --- 3. Test Keyword Search ---
    println!("\n--- Step 2: Testing Keyword Search ---");
    let keyword_res = client
        .post(format!("{app_address}/search/keyword"))
        .json(&json!({ "query": "PostgreSQL" }))
        .send()
        .await?;
    let keyword_results: Vec<serde_json::Value> = keyword_res.json().await?;
    assert_eq!(keyword_results.len(), 1);
    let top_keyword_title = keyword_results[0]["title"].as_str().unwrap();
    println!("-> Keyword search for 'PostgreSQL' found: '{top_keyword_title}'");
    assert!(top_keyword_title.contains("PostgreSQL"));

    // --- 4. Test Vector Search (with controlled mock) ---
    println!("\n--- Step 3: Testing Vector Search ---");
    // This query will be mapped to the `rust_vec` by our mock.
    let vector_res = client
        .post(format!("{app_address}/search/vector"))
        .json(&json!({ "query": "creating websites with rust" }))
        .send()
        .await?;
    let vector_results: Vec<serde_json::Value> = vector_res.json().await?;
    assert_eq!(vector_results.len(), 1);
    let top_vector_title = vector_results[0]["title"].as_str().unwrap();
    println!("-> Vector search for 'creating websites with rust' found: '{top_vector_title}'");
    assert!(top_vector_title.contains("Web Apps with Rust"));
    // The score should be 0.0 because the query vector and document vector are identical.
    assert_eq!(vector_results[0]["score"].as_f64().unwrap(), 0.0);

    // --- 5. Test Hybrid Search (the original failing case) ---
    println!("\n--- Step 4: Testing Hybrid Search ---");
    // This query will be mapped to the `qwen3_vec`.
    let hybrid_res = client
        .post(format!("{app_address}/search/hybrid"))
        .json(&json!({ "query": "Qwen3" }))
        .send()
        .await?;
    let hybrid_results: Vec<serde_json::Value> = hybrid_res.json().await?;
    assert!(!hybrid_results.is_empty());
    let top_hybrid_title = hybrid_results[0]["title"].as_str().unwrap();
    println!("-> Hybrid search for 'Qwen3' found: '{top_hybrid_title}'");
    // With the mock, the vector search part is now guaranteed to find the Qwen3 article,
    // so the hybrid search will correctly rank it first.
    assert!(top_hybrid_title.contains("Qwen3"));

    // --- 6. Assert mock call counts ---
    // Embedding phase: 1 call for each of the 3 articles.
    // Search phase: 1 call for vector search (rust), 1 call for hybrid search (qwen3).
    postgres_mock.assert_hits(1); // Only hit during embedding
    qwen3_mock.assert_hits(2); // Hit during embedding and hybrid search
    rust_mock.assert_hits(2); // Hit during embedding and vector search

    Ok(())
}
