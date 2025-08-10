//! # End-to-End Search Workflow Test
//!
//! This file contains a comprehensive integration test that simulates the full
//! user workflow: ingesting articles from a live RSS feed, generating embeddings
//! for them using the configured provider, and then performing a vector
//! similarity search to find relevant content.

use anyhow::Result;

use httpmock::prelude::*;
use reqwest::Client;
use serde_json::json;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};
use turso::Value as TursoValue;

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

/// This is a full end-to-end test that performs the following steps:
/// 1. Starts the server with a temporary, empty database.
/// 2. Can ingests articles from a live RSS feed (https://news.smol.ai/rss.xml).
/// 3. Finds all newly ingested articles that don't have an embedding.
/// 4. Calls the `/embed` endpoint for each new article, which uses the configured
///    (real) embedding API to generate and save the vector.
/// 5. Calls the `/search` endpoint with a query, which also hits the embedding API.
/// 6. Asserts that the search returns relevant results.
///
#[tokio::test]
async fn test_e2e_full_ingest_embed_search_flow() -> Result<()> {
    // --- 1. Arrange ---
    let temp_db_file = NamedTempFile::new()?;
    let db_path = temp_db_file.path().to_path_buf();
    let app_address = spawn_app_for_e2e_test(db_path.clone()).await?;
    let client = Client::new();

    // Set up a mock server for the RSS feed. This makes the test fast, reliable,
    // and independent of external network conditions.
    let server = MockServer::start();
    let mock_rss_content = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <rss version="2.0">
        <channel>
            <title>Mock Feed</title>
            <link>http://mock.com</link>
            <description>A mock feed for testing.</description>
            <item>
                <title>Organic Gardening Tips</title>
                <link>http://mock.com/gardening</link>
                <description>How to grow vegetables without any pesticides.</description>
                <pubDate>Mon, 01 Jan 2024 12:00:00 GMT</pubDate>
            </item>
            <item>
                <title>The Rise of Qwen3</title>
                <link>http://mock.com/qwen3</link>
                <description>Exploring the new features of the Qwen3 large language model.</description>
                <pubDate>Wed, 03 Jan 2024 12:00:00 GMT</pubDate>
            </item>
            <item>
                <title>Baking Sourdough Bread</title>
                <link>http://mock.com/baking</link>
                <description>A step-by-step guide for beginners to make delicious bread.</description>
                <pubDate>Tue, 02 Jan 2024 12:00:00 GMT</pubDate>
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

    // --- 2. Act: Ingest ---
    println!("Step 1/3: Ingesting articles from {rss_feed_url}...");
    let ingest_res = client
        .post(format!("{app_address}/ingest"))
        .json(&json!({ "url": rss_feed_url }))
        .send()
        .await?;
    assert!(
        ingest_res.status().is_success(),
        "Failed to ingest RSS feed."
    );
    let ingest_body: serde_json::Value = ingest_res.json().await?;
    let ingested_count: usize = ingest_body["ingested_articles"].as_u64().unwrap_or(0) as usize;
    println!("-> Ingestion successful. Found {ingested_count} new articles.");
    assert_eq!(
        ingested_count, 3,
        "Expected to ingest exactly 3 articles from the mock feed."
    );

    // --- DEBUG: Log newest articles to confirm order ---
    println!("-> Verifying newest articles in DB before embedding...");
    let db = turso::Builder::new_local(db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;
    let mut stmt = conn
        .prepare("SELECT id, title, pub_date FROM articles ORDER BY pub_date DESC LIMIT 3")
        .await?;
    let mut rows = stmt.query(()).await?;
    while let Some(row) = rows.next().await? {
        let id = match row.get_value(0)? {
            TursoValue::Integer(i) => i,
            _ => panic!("Expected integer for ID"),
        };
        let title = match row.get_value(1)? {
            TursoValue::Text(s) => s,
            _ => panic!("Expected text for title"),
        };
        let pub_date = match row.get_value(2)? {
            TursoValue::Text(s) => s,
            _ => panic!("Expected text for pub_date"),
        };
        println!("   - ID: {id}, Title: '{title}', PubDate: {pub_date}");
    }

    // --- 3. Act: Embed ---
    println!("Step 2/3: Embedding the 3 newest articles...");
    let embed_res = client
        .post(format!("{app_address}/embed/new"))
        .json(&json!({ "limit": 3 }))
        .send()
        .await?;
    assert!(
        embed_res.status().is_success(),
        "Failed to call the /embed/new endpoint."
    );
    let embed_body: serde_json::Value = embed_res.json().await?;
    println!(
        "-> Embedding successful. Processed {} articles.",
        embed_body["embedded_articles"]
    );

    // --- 4. Act: Search ---
    let search_query = "Qwen3";
    println!("Step 3/3: Performing vector search for query: '{search_query}'...");
    let search_res = client
        .post(format!("{app_address}/search"))
        .json(&json!({ "query": search_query }))
        .send()
        .await?;

    // --- 5. Assert ---
    assert!(
        search_res.status().is_success(),
        "Search request failed. Response: {:?}",
        search_res.text().await?
    );

    let search_results: Vec<serde_json::Value> = search_res.json().await?;
    println!("-> Search returned {} results.", search_results.len());

    // --- DEBUG: Log all search results for inspection ---
    println!(
        "-> Full search results:\n{}",
        serde_json::to_string_pretty(&search_results)?
    );

    assert!(
        !search_results.is_empty(),
        "Search for '{search_query}' returned no results."
    );

    println!(
        "-> Checking for relevance in {} results...",
        search_results.len()
    );
    let is_relevant = search_results.iter().any(|result| {
        let title = result["title"].as_str().unwrap_or_default().to_lowercase();
        let description = result["description"]
            .as_str()
            .unwrap_or_default()
            .to_lowercase();
        title.contains("qwen") || description.contains("qwen")
    });

    assert!(
        is_relevant,
        "None of the top search results for '{search_query}' contained the keyword 'qwen' in their title or description."
    );

    Ok(())
}
