//! # Ingest Endpoint Tests
//!
//! This file contains integration tests for the `/ingest` endpoint,
//! verifying that it correctly fetches an RSS feed and stores the articles
//! in the database with the correct ownership.

mod common;

use anyhow::Result;
use common::{generate_jwt, TestApp};
use core_access::get_or_create_user;
use httpmock::Method;
use serde_json::json;
use turso::{Builder, Value as TursoValue};

#[tokio::test]
async fn test_ingest_endpoint_success() -> Result<()> {
    // --- Arrange ---
    let app = TestApp::spawn().await?;
    let user_identifier = "ingest-test-user@example.com";
    let token = generate_jwt(user_identifier)?;

    // Set up a mock server for the RSS feed.
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

    // Call the /ingest endpoint on our app server with authentication.
    let response = app
        .client
        .post(format!("{}/ingest/rss", app.address))
        .bearer_auth(token)
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
    assert_eq!(response_body["result"]["ingested_articles"], 1);
    rss_mock.assert();

    // --- Assert (Database State) ---
    // Verify the data was written to the database correctly.
    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await
        .expect("Failed to connect to temp db");
    let conn = db.connect().expect("Failed to get connection from db");

    // Get the expected owner ID.
    let expected_user = get_or_create_user(&db, user_identifier).await?;

    // Check the content and owner of the inserted document.
    let mut result_set = conn
        .query(
            "SELECT owner_id, title FROM documents WHERE source_url = 'http://mock.com/article1'",
            (),
        )
        .await
        .expect("Failed to query db for specific document");
    let row = result_set
        .next()
        .await?
        .expect("Row for document 1 is None");
    let owner_id: String = match row.get_value(0)? {
        TursoValue::Text(s) => s,
        other => panic!("Expected Text for owner_id, got {other:?}"),
    };
    let title: String = match row.get_value(1)? {
        TursoValue::Text(s) => s,
        other => panic!("Expected Text for title, got {other:?}"),
    };
    assert_eq!(
        owner_id, expected_user.id,
        "The owner_id in the DB is incorrect."
    );
    assert_eq!(title, "Test Article 1");

    Ok(())
}
