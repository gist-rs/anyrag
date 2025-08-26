//! # Text Ingest Endpoint Tests
//!
//! This file contains integration tests for the `POST /ingest/text` endpoint.
//! It verifies that the server can accept raw text, chunk it correctly
//! according to the library's logic, and store it in the database.

use anyhow::Result;
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
async fn test_ingest_text_endpoint_success() -> Result<()> {
    // --- Arrange ---

    // 1. Create a temporary database file that will be deleted automatically.
    let temp_db_file = NamedTempFile::new().expect("Failed to create temp db file");
    let db_path = temp_db_file.path().to_path_buf();

    // 2. Spawn the application, configured to use our temporary database.
    let app_address = spawn_app_with_db(db_path.clone()).await?;

    // 3. Define the text payload. It includes a short paragraph and one that
    // will be split into two chunks by the chunking logic (4096 limit).
    let long_paragraph = "a".repeat(5000);
    let text_to_ingest = format!("This is the first paragraph.\n\n{long_paragraph}");
    let payload = json!({
        "text": text_to_ingest,
        "source": "manual_test"
    });

    // --- Act ---

    // 4. Call the /ingest/text endpoint on our app server.
    let client = Client::new();
    let response = client
        .post(format!("{app_address}/ingest/text"))
        .json(&payload)
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
    // We expect 3 chunks: one for the short paragraph, two for the long one.
    assert_eq!(response_body["result"]["ingested_chunks"], 3);

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
    assert_eq!(count, 3, "The number of articles in the DB should be 3.");

    // Check the content of the first chunk (the short paragraph).
    let mut result_set = conn
        .query(
            "SELECT description FROM articles WHERE source_url = 'manual_test' ORDER BY id ASC LIMIT 1",
            (),
        )
        .await
        .expect("Failed to query db for specific article");
    let row = result_set
        .next()
        .await
        .expect("Failed to get row for article 1")
        .expect("Row for article 1 is None");
    let description: String = match row.get_value(0).unwrap() {
        TursoValue::Text(s) => s,
        other => panic!("Expected Text for description, got {other:?}"),
    };
    assert_eq!(description, "This is the first paragraph.");

    Ok(())
}
