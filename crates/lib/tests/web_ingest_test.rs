//! # Web Ingestion Tests
//!
//! This file contains tests for the web content fetching logic,
//! specifically for the different `WebIngestStrategy` options.

mod common;

use anyrag::ingest::knowledge::{fetch_web_content, KnowledgeError, WebIngestStrategy};
use common::setup_tracing;
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_fetch_web_content_raw_html() {
    // --- 1. Arrange ---
    setup_tracing();
    let server = MockServer::start().await;
    let html_content =
        "<html><head><title>Test</title></head><body><h1>Hello</h1><p>This is a test.</p></body></html>";
    let expected_markdown = "Test\n\nHello\n==========\n\nThis is a test.";

    Mock::given(method("GET"))
        .and(path("/test_html"))
        .respond_with(ResponseTemplate::new(200).set_body_string(html_content))
        .mount(&server)
        .await;

    // --- 2. Act ---
    let result = fetch_web_content(
        Url::parse(&server.uri())
            .unwrap()
            .join("/test_html")
            .unwrap()
            .as_ref(),
        WebIngestStrategy::RawHtml,
    )
    .await;

    // --- 3. Assert ---
    assert!(
        result.is_ok(),
        "fetch_web_content failed: {:?}",
        result.err()
    );
    let markdown = result.unwrap();
    assert_eq!(markdown.trim(), expected_markdown.trim());
}

#[tokio::test]
async fn test_fetch_web_content_direct_markdown() {
    // --- 1. Arrange ---
    setup_tracing();
    let server = MockServer::start().await;
    let markdown_content = "# Markdown File\n\nThis is a raw markdown file.";

    Mock::given(method("GET"))
        .and(path("/test.md"))
        .respond_with(ResponseTemplate::new(200).set_body_string(markdown_content))
        .mount(&server)
        .await;

    // --- 2. Act ---
    // The strategy doesn't matter here, as it should always fetch .md files directly.
    let result = fetch_web_content(
        Url::parse(&server.uri())
            .unwrap()
            .join("/test.md")
            .unwrap()
            .as_ref(),
        WebIngestStrategy::RawHtml,
    )
    .await;

    // --- 3. Assert ---
    assert!(
        result.is_ok(),
        "fetch_web_content for .md failed: {:?}",
        result.err()
    );
    let markdown = result.unwrap();
    assert_eq!(markdown, markdown_content);
}

#[tokio::test]
async fn test_fetch_web_content_raw_html_error_status() {
    // --- 1. Arrange ---
    setup_tracing();
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/notfound"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&server)
        .await;

    // --- 2. Act ---
    let result = fetch_web_content(
        Url::parse(&server.uri())
            .unwrap()
            .join("/notfound")
            .unwrap()
            .as_ref(),
        WebIngestStrategy::RawHtml,
    )
    .await;

    // --- 3. Assert ---
    assert!(result.is_err());
    match result.err().unwrap() {
        KnowledgeError::Html(e) => {
            assert!(e.contains("status 404"));
            assert!(e.contains("Not Found"));
        }
        other => panic!("Expected Html error, but got {other:?}"),
    }
}
