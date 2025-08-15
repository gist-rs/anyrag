//! # Embedding and Search Endpoint Tests
//!
//! This file contains integration tests for the `/embed` and `/search` endpoints.
//! It verifies the complete flow: ingesting an article, generating an embedding for it,
//! storing the embedding, and then retrieving the article via a vector similarity search.

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

use main::types::ApiResponse;

/// Spawns the application in the background for testing, configured with a temporary DB and mock APIs.
async fn spawn_app_for_embedding_test(
    db_path: PathBuf,
    embeddings_api_url: String,
) -> Result<String> {
    dotenvy::dotenv().ok();
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .compact()
        .try_init();

    // Set environment variables for the test instance.
    std::env::set_var("EMBEDDINGS_API_URL", embeddings_api_url);
    std::env::set_var("EMBEDDINGS_MODEL", "test-embedding-model");

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

    sleep(Duration::from_millis(100)).await;

    Ok(address)
}

#[tokio::test]
async fn test_embed_and_search_flow() -> Result<()> {
    // --- 1. Arrange ---
    let temp_db_file = NamedTempFile::new()?;
    let db_path = temp_db_file.path().to_path_buf();

    let server = MockServer::start();
    let rss_mock = server.mock(|when, then| {
        when.method(GET).path("/rss");
        then.status(200)
            .header("Content-Type", "application/rss+xml")
            .body(r#"<rss version="2.0"><channel><title>Mock</title><link>http://m.com</link><item><title>Test Article 1</title><link>http://m.com/1</link><description>Summary 1</description></item></channel></rss>"#);
    });

    let mock_vector = vec![0.1, 0.2, 0.3, 0.4];
    let embeddings_mock = server.mock(|when, then| {
        when.method(POST).path("/v1/embeddings");
        then.status(200)
            .header("Content-Type", "application/json")
            .json_body(json!({
                "data": [{ "embedding": mock_vector }]
            }));
    });

    let app_address =
        spawn_app_for_embedding_test(db_path.clone(), server.url("/v1/embeddings")).await?;
    let client = Client::new();

    // --- 2. Act & Assert: Ingest ---
    let ingest_res = client
        .post(format!("{app_address}/ingest"))
        .json(&json!({ "url": server.url("/rss") }))
        .send()
        .await?;
    assert!(ingest_res.status().is_success());
    rss_mock.assert();

    // --- 3. Act & Assert: Embed ---
    let db = turso::Builder::new_local(db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;
    let mut stmt = conn
        .prepare("SELECT id FROM articles WHERE link = 'http://m.com/1'")
        .await?;
    let mut rows = stmt.query(()).await?;
    let article_id: i64 = if let Some(row) = rows.next().await? {
        match row.get_value(0)? {
            TursoValue::Integer(i) => i,
            _ => panic!("Expected integer for article ID"),
        }
    } else {
        panic!("Article not found in database after ingest");
    };

    let embed_res = client
        .post(format!("{app_address}/embed"))
        .json(&json!({ "article_id": article_id }))
        .send()
        .await?;
    assert!(embed_res.status().is_success());

    // --- 4. Act & Assert: Search ---
    let search_res = client
        .post(format!("{app_address}/search/vector"))
        .json(&json!({ "query": "A query about the test article" }))
        .send()
        .await?;
    assert!(search_res.status().is_success());

    embeddings_mock.assert_hits(2);

    let response: ApiResponse<Vec<serde_json::Value>> = search_res.json().await?;
    let search_results = response.result;
    assert_eq!(search_results.len(), 1);
    let top_result = &search_results[0];
    assert_eq!(top_result["title"], "Test Article 1");
    assert_eq!(top_result["link"], "http://m.com/1");
    assert_eq!(top_result["score"], 0.0);

    Ok(())
}
