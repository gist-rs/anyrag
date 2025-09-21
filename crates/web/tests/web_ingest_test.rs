//! # Web Ingestion Tests
//!
//! This file contains tests for the web content fetching logic,
//! specifically for the different `WebIngestStrategy` options.

use anyrag_web::{fetch_web_content, WebIngestError, WebIngestStrategy};
use std::sync::Once;
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

static INIT: Once = Once::new();

/// Initializes tracing for tests.
pub fn setup_tracing() {
    INIT.call_once(|| {
        tracing_subscriber::fmt::init();
    });
}

#[tokio::test]
async fn test_fetch_web_content_raw_html() {
    // --- 1. Arrange ---
    setup_tracing();
    let server = MockServer::start().await;
    let html_content =
        "<html><head><title>Test</title></head><body><h1>Hello</h1><p>This is a test.</p></body></html>";

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
    assert!(markdown.contains("# Test"));
    assert!(markdown.contains("This is a test."));
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
        WebIngestError::Html(e) => {
            assert!(e.contains("status 404"));
            assert!(e.contains("Not Found"));
        }
        other => panic!("Expected Html error, but got {other:?}"),
    }
}
