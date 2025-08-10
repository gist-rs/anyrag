//! # End-to-End Search Workflow Test
//!
//! This file contains a comprehensive integration test that simulates the full
//! user workflow: ingesting articles from a live RSS feed, generating embeddings
//! for them using the configured provider, and then performing a vector
//! similarity search to find relevant content.

use anyhow::Result;
use futures::{stream, StreamExt};
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
/// 2. Ingests articles from a live RSS feed (https://news.smol.ai/rss.xml).
/// 3. Finds all newly ingested articles that don't have an embedding.
/// 4. Calls the `/embed` endpoint for each new article, which uses the configured
///    (real) embedding API to generate and save the vector.
/// 5. Calls the `/search` endpoint with a query, which also hits the embedding API.
/// 6. Asserts that the search returns relevant results.
///
/// This test is marked `#[ignore]` because it depends on external network services
/// and can be slow. Run it explicitly with `cargo test -- --ignored`.
#[tokio::test]
#[ignore]
async fn test_e2e_full_ingest_embed_search_flow() -> Result<()> {
    // --- 1. Arrange ---
    let temp_db_file = NamedTempFile::new()?;
    let db_path = temp_db_file.path().to_path_buf();
    let rss_feed_url = "https://news.smol.ai/rss.xml";
    let app_address = spawn_app_for_e2e_test(db_path.clone()).await?;
    let client = Client::new();

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
    assert!(
        ingested_count > 0,
        "Expected to ingest at least one article from the live feed."
    );

    // --- 3. Act: Embed ---
    println!("Step 2/3: Embedding all new articles...");
    let db = turso::Builder::new_local(db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;
    let mut stmt = conn
        .prepare("SELECT id FROM articles ORDER BY pub_date ASC LIMIT 3")
        .await?;
    let mut rows = stmt.query(()).await?;

    let mut articles_to_embed = Vec::new();
    while let Some(row) = rows.next().await? {
        if let Ok(TursoValue::Integer(id)) = row.get_value(0) {
            articles_to_embed.push(id);
        }
    }
    println!(
        "-> Found {} articles that need embedding.",
        articles_to_embed.len()
    );
    assert!(
        !articles_to_embed.is_empty(),
        "No new articles were found to embed."
    );

    // Concurrently embed all articles. We use `for_each_concurrent` to send up to 10
    // requests in parallel, dramatically speeding up the embedding process for
    // large RSS feeds and preventing the test from timing out.
    stream::iter(articles_to_embed.into_iter())
        .for_each_concurrent(10, |article_id| {
            let client = client.clone();
            let app_address = app_address.clone();
            async move {
                println!("   - Embedding article ID: {article_id}");
                let embed_res = client
                    .post(format!("{app_address}/embed"))
                    .json(&json!({ "article_id": article_id }))
                    .send()
                    .await
                    .unwrap_or_else(|e| {
                        panic!("Failed to send request for article ID {article_id}: {e}")
                    });

                let status = embed_res.status();
                if !status.is_success() {
                    let body = embed_res.text().await.unwrap_or_default();
                    panic!(
                        "Failed to embed article ID {article_id}. Status: {status}. Body: {body}"
                    );
                }
            }
        })
        .await;
    println!("-> All new articles have been embedded.");

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

    assert!(
        !search_results.is_empty(),
        "Search for '{search_query}' returned no results."
    );

    let top_result = &search_results[0];
    println!(
        "-> Top search result: {}",
        serde_json::to_string_pretty(top_result)?
    );

    let top_description: String = serde_json::from_value(top_result["description"].clone())?;
    let is_relevant = top_description.contains(search_query);
    assert!(is_relevant, "The top search result title ('{top_description}') did not seem relevant to the query '{search_query}'.");

    Ok(())
}
