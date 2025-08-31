//! # Ingest Endpoint Tests
//!
//! This file contains integration tests for the `/ingest` endpoint,
//! verifying that it correctly fetches an RSS feed and stores the articles
//! in the database.

mod common;

use anyhow::Result;
use common::TestApp;
use httpmock::Method;
use serde_json::json;
use turso::Value as TursoValue;

#[tokio::test]
async fn test_ingest_endpoint_success() -> Result<()> {
    // --- Arrange ---
    let app = TestApp::spawn().await?;

    // 3. Set up a mock server for the RSS feed.
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
    let rss_mock = app.mock_server.mock(|when, then| {
        when.method(Method::GET).path("/rss");
        then.status(200)
            .header("content-type", "application/rss+xml")
            .body(mock_rss_content);
    });
    let mock_rss_url = app.mock_server.url("/rss");

    // --- Act ---

    // 4. Call the /ingest endpoint on our app server.
    let response = app
        .client
        .post(format!("{}/ingest", app.address))
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
    assert_eq!(response_body["result"]["ingested_articles"], 2);
    rss_mock.assert();

    // --- Assert (Database State) ---
    // Verify the data was written to the database correctly.
    let db = turso::Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await
        .expect("Failed to connect to temp db");
    let conn = db.connect().expect("Failed to get connection from db");

    // Check the total count of articles.
    let mut result_set = conn
        .query("SELECT COUNT(*) FROM documents", ())
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
    assert_eq!(count, 2, "The number of documents in the DB is incorrect.");

    // Check the content of one of the documents to be sure.
    let mut result_set = conn
        .query(
            "SELECT title FROM documents WHERE source_url = 'http://mock.com/article1'",
            (),
        )
        .await
        .expect("Failed to query db for specific document");
    let row = result_set
        .next()
        .await
        .expect("Failed to get row for document 1")
        .expect("Row for document 1 is None");
    let title: String = match row.get_value(0).unwrap() {
        TursoValue::Text(s) => s,
        other => panic!("Expected Text for title, got {other:?}"),
    };
    assert_eq!(title, "Test Article 1");

    Ok(())
}
