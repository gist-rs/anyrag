//! # Ingest Endpoint Tests
//!
//! This file contains integration tests for the `/ingest` endpoint,
//! verifying that it correctly fetches an RSS feed and stores the articles
//! in the database.

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

/// Spawns the application in the background for testing, using a specific database file.
async fn spawn_app_with_db(db_path: PathBuf) -> Result<String> {
    dotenvy::dotenv().ok();
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .compact()
        .try_init();

    // Load configuration, but override the db_url.
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
async fn test_ingest_endpoint_success() -> Result<()> {
    // --- Arrange ---

    // 1. Create a temporary database file that will be deleted automatically.
    let temp_db_file = NamedTempFile::new().expect("Failed to create temp db file");
    let db_path = temp_db_file.path().to_path_buf();

    // 2. Spawn the application, configured to use our temporary database.
    let app_address = spawn_app_with_db(db_path.clone()).await?;

    // 3. Set up a mock server for the RSS feed.
    let server = MockServer::start();
    let mock_rss_content = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <rss version="2.0">
        <channel>
            <title>Mock Feed</title>
            <link>http://mock.com</link>
            <description>A mock feed for testing.</description>
            <item>
                <title>Test Article 1</title>
                <link>http://mock.com/article1</link>
                <description>Summary of article 1.</description>
                <pubDate>Mon, 01 Jan 2024 12:00:00 GMT</pubDate>
            </item>
            <item>
                <title>Test Article 2</title>
                <link>http://mock.com/article2</link>
                <description>Summary of article 2.</description>
                <pubDate>Tue, 02 Jan 2024 12:00:00 GMT</pubDate>
            </item>
        </channel>
        </rss>
    "#;
    let rss_mock = server.mock(|when, then| {
        when.method(GET).path("/rss");
        then.status(200)
            .header("content-type", "application/rss+xml")
            .body(mock_rss_content);
    });
    let mock_rss_url = server.url("/rss");

    // --- Act ---

    // 4. Call the /ingest endpoint on our app server.
    let client = Client::new();
    let response = client
        .post(format!("{app_address}/ingest"))
        .json(&json!({ "url": mock_rss_url }))
        .send()
        .await
        .expect("Failed to execute request.");

    // --- Assert (API Response) ---
    assert!(
        response.status().is_success(),
        "Request failed with status: {}",
        response.status()
    );
    let response_body: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse response JSON");
    assert_eq!(response_body["ingested_articles"], 2);
    rss_mock.assert();

    // --- Assert (Database State) ---
    // Verify the data was written to the database correctly.
    let db = turso::Builder::new_local(db_path.to_str().unwrap())
        .build()
        .await
        .expect("Failed to connect to temp db");
    let conn = db.connect().expect("Failed to get connection from db");

    // Check the total count of articles.
    let mut result_set = conn
        .query("SELECT COUNT(*) FROM articles", ())
        .await
        .expect("Failed to query db for count");
    let row = result_set
        .next()
        .await
        .expect("Failed to get next row")
        .expect("Row is None");

    let count: i64 = match row.get_value(0).unwrap() {
        TursoValue::Integer(i) => i,
        other => panic!("Expected Integer, got {other:?}"),
    };
    assert_eq!(count, 2, "The number of articles in the DB is incorrect.");

    // Check the content of one of the articles to be sure.
    let mut result_set = conn
        .query(
            "SELECT title FROM articles WHERE link = 'http://mock.com/article1'",
            (),
        )
        .await
        .expect("Failed to query db for specific article");
    let row = result_set
        .next()
        .await
        .expect("Failed to get row for article 1")
        .expect("Row for article 1 is None");
    let title: String = match row.get_value(0).unwrap() {
        TursoValue::Text(s) => s,
        other => panic!("Expected Text for title, got {other:?}"),
    };
    assert_eq!(title, "Test Article 1");

    Ok(())
}
