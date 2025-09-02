//! # Embedding and Search Endpoint Tests
//!
//! This file contains integration tests for the `/embed` and `/search` endpoints.
//! It verifies the complete flow: ingesting an article, generating an embedding for it,
//! storing the embedding, and then retrieving the article via a vector similarity search.

mod common;

use anyhow::Result;
use common::TestApp;
use httpmock::Method;
use serde_json::json;

use anyrag_server::types::ApiResponse;

#[tokio::test]
async fn test_embed_and_search_flow() -> Result<()> {
    // --- 1. Arrange ---
    let app = TestApp::spawn().await?;

    let rss_mock = app.mock_server.mock(|when, then| {
        when.method(Method::GET).path("/rss");
        then.status(200)
            .header("Content-Type", "application/rss+xml")
            .body(r#"<rss version="2.0"><channel><title>Mock</title><link>http://m.com</link><item><title>Test Article 1</title><link>http://m.com/1</link><description>Summary 1</description></item></channel></rss>"#);
    });

    let mock_vector = vec![0.1, 0.2, 0.3, 0.4];
    let embeddings_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .header("Content-Type", "application/json")
            .json_body(json!({
                "data": [{ "embedding": mock_vector }]
            }));
    });

    // --- 2. Act & Assert: Ingest ---
    let ingest_res = app
        .client
        .post(format!("{}/ingest", app.address))
        .json(&json!({ "url": app.mock_server.url("/rss") }))
        .send()
        .await?;
    assert!(ingest_res.status().is_success());
    rss_mock.assert();

    // --- 3. Act & Assert: Embed ---
    let embed_res = app
        .client
        .post(format!("{}/embed/new", app.address))
        .json(&json!({ "limit": 10 }))
        .send()
        .await?;
    assert!(embed_res.status().is_success());

    // --- 4. Act & Assert: Search ---
    let search_res = app
        .client
        .post(format!("{}/search/vector", app.address))
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
    assert_eq!(top_result["score"], 1.0);

    Ok(())
}
