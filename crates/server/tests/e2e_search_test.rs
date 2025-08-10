//! # End-to-End Search Workflow Tests
//!
//! This file contains comprehensive integration tests that simulate the full
//! user workflow: ingesting articles, embedding them, and then using the
//! various search endpoints to find them.

use anyhow::Result;
use httpmock::prelude::*;
use reqwest::Client;
use serde_json::json;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};

// Include the binary's main source file to access its components.
#[path = "../src/main.rs"]
mod main;

/// Spawns the application in the background for testing, configured with a temporary DB.
/// It uses the embedding API configuration from the environment.
async fn spawn_app_for_e2e_test(db_path: PathBuf) -> Result<String> {
    dotenvy::dotenv().ok();
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .compact()
        .try_init();

    // Load configuration, but override the db_url to use our temporary database.
    let mut config = main::config::get_config().expect("Failed to load test configuration");
    config.db_url = db_path
        .to_str()
        .expect("Failed to convert temp db path to string")
        .to_string();

    // Make sure the embedding environment variables are set.
    assert!(
        config.embeddings_api_url.is_some(),
        "EMBEDDINGS_API_URL must be set in .env for this test"
    );
    assert!(
        config.embeddings_model.is_some(),
        "EMBEDDINGS_MODEL must be set in .env for this test"
    );

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
async fn test_e2e_all_search_endpoints() -> Result<()> {
    // --- 1. Arrange ---
    let temp_db_file = NamedTempFile::new()?;
    let db_path = temp_db_file.path().to_path_buf();
    let app_address = spawn_app_for_e2e_test(db_path.clone()).await?;
    let client = Client::new();

    // Set up a mock server for a predictable RSS feed.
    let server = MockServer::start();
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
                <title>Building Web Apps with Python</title>
                <link>http://mock.com/python-web</link>
                <description>A tutorial on creating interactive sites using FastHTML.</description>
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
        .json(&json!({ "limit": 3 }))
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
    assert!(!keyword_results.is_empty());
    let top_keyword_title = keyword_results[0]["title"].as_str().unwrap();
    println!("-> Keyword search for 'PostgreSQL' found: '{top_keyword_title}'");
    assert!(top_keyword_title.contains("PostgreSQL"));

    // --- 4. Test Vector Search ---
    println!("\n--- Step 3: Testing Vector Search ---");
    let vector_res = client
        .post(format!("{app_address}/search/vector"))
        .json(&json!({ "query": "creating websites with python" }))
        .send()
        .await?;
    let vector_results: Vec<serde_json::Value> = vector_res.json().await?;
    assert!(!vector_results.is_empty());
    let top_vector_title = vector_results[0]["title"].as_str().unwrap();
    println!("-> Vector search for 'creating websites with python' found: '{top_vector_title}'");
    assert!(top_vector_title.contains("Web Apps with Python"));

    // --- 5. Test Hybrid Search ---
    println!("\n--- Step 4: Testing Hybrid Search ---");
    let hybrid_res = client
        .post(format!("{app_address}/search/hybrid"))
        .json(&json!({ "query": "Qwen3" }))
        .send()
        .await?;
    let hybrid_results: Vec<serde_json::Value> = hybrid_res.json().await?;
    assert!(!hybrid_results.is_empty());
    let top_hybrid_title = hybrid_results[0]["title"].as_str().unwrap();
    println!("-> Hybrid search for 'Qwen3 language model' found: '{top_hybrid_title}'");
    assert!(top_hybrid_title.contains("Qwen3"));

    Ok(())
}
