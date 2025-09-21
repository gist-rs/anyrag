//! # RSS Crate Tests
//!
//! This file contains integration tests for the `anyrag-rss` crate,
//! ensuring that the RSS feed fetching and parsing logic works as expected,
//! independent of the main server.

use anyhow::Result;
use anyrag::ingest::{IngestError, Ingestor};
use anyrag_rss::RssIngestor;
use anyrag_test_utils::TestSetup;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper function to create a mock RSS feed.
fn mock_rss_feed_content() -> String {
    r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <rss version="2.0">
        <channel>
            <title>Test Feed</title>
            <link>http://localhost/test</link>
            <description>A feed for testing the RSS ingestor.</description>
            <item>
                <title>Article One</title>
                <link>http://localhost/test/article1</link>
                <description>This is the first article.</description>
            </item>
            <item>
                <title>Article Two</title>
                <link>http://localhost/test/article2</link>
                <description>This is the second article.</description>
            </item>
        </channel>
        </rss>
    "#
    .to_string()
}

#[tokio::test]
async fn test_rss_ingestor_success() -> Result<()> {
    // --- Arrange ---
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(mock_rss_feed_content())
                .insert_header("Content-Type", "application/rss+xml"),
        )
        .mount(&server)
        .await;

    let setup = TestSetup::new().await?;
    let ingestor = RssIngestor::new(&setup.db);
    let owner_id = "rss-user@test.com";
    let source = json!({ "url": server.uri() + "/feed.xml" }).to_string();

    // --- Act ---
    let result = ingestor.ingest(&source, Some(owner_id)).await?;

    // --- Assert ---
    assert_eq!(result.documents_added, 2);
    assert_eq!(result.document_ids.len(), 2);
    assert_eq!(result.source, server.uri() + "/feed.xml");

    let conn = setup.db.connect()?;
    let count: i64 = conn
        .query(
            "SELECT COUNT(*) FROM documents WHERE owner_id = ?",
            [owner_id],
        )
        .await?
        .next()
        .await?
        .unwrap()
        .get(0)?;
    assert_eq!(count, 2);

    let title: String = conn
        .query(
            "SELECT title FROM documents WHERE source_url = 'http://localhost/test/article2'",
            (),
        )
        .await?
        .next()
        .await?
        .unwrap()
        .get(0)?;
    assert_eq!(title, "Article Two");

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_rss_ingestor_idempotency() -> Result<()> {
    // --- Arrange ---
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(mock_rss_feed_content())
                .insert_header("Content-Type", "application/rss+xml"),
        )
        .mount(&server)
        .await;

    let setup = TestSetup::new().await?;
    let ingestor = RssIngestor::new(&setup.db);
    let owner_id = "rss-user@test.com";
    let source = json!({ "url": server.uri() + "/feed.xml" }).to_string();

    // --- Act ---
    // First ingest
    let result1 = ingestor.ingest(&source, Some(owner_id)).await?;
    // Second ingest of the same feed
    let result2 = ingestor.ingest(&source, Some(owner_id)).await?;

    // --- Assert ---
    assert_eq!(result1.documents_added, 2);
    assert_eq!(result2.documents_added, 0); // No new documents should be added
    assert!(result2.document_ids.is_empty());

    let conn = setup.db.connect()?;
    let count: i64 = conn
        .query("SELECT COUNT(*) FROM documents", ())
        .await?
        .next()
        .await?
        .unwrap()
        .get(0)?;
    assert_eq!(count, 2); // Total count remains 2

    Ok(())
}

#[tokio::test]
async fn test_rss_ingestor_fetch_error() -> Result<()> {
    // --- Arrange ---
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let setup = TestSetup::new().await?;
    let ingestor = RssIngestor::new(&setup.db);
    let source = json!({ "url": server.uri() + "/feed.xml" }).to_string();

    // --- Act ---
    let result = ingestor.ingest(&source, None).await;

    // --- Assert ---
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), IngestError::Fetch(_)));

    Ok(())
}

#[tokio::test]
async fn test_rss_ingestor_parse_error() -> Result<()> {
    // --- Arrange ---
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("this is not valid xml")
                .insert_header("Content-Type", "application/rss+xml"),
        )
        .mount(&server)
        .await;

    let setup = TestSetup::new().await?;
    let ingestor = RssIngestor::new(&setup.db);
    let source = json!({ "url": server.uri() + "/feed.xml" }).to_string();

    // --- Act ---
    let result = ingestor.ingest(&source, None).await;

    // --- Assert ---
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), IngestError::Parse(_)));

    Ok(())
}
