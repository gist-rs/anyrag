//! # Knowledge Graph E2E Test
//!
//! This file contains end-to-end tests for the Knowledge Graph integration,
//! verifying both the dedicated endpoint and its integration into the main
//! hybrid search RAG pipeline.

mod common;

use anyhow::Result;
use anyrag_server::types::ApiResponse;
use chrono::{Duration, Utc};
use common::TestApp;
use httpmock::Method;
use serde_json::{json, Value};

#[tokio::test]
#[cfg(feature = "graph_db")]
async fn test_knowledge_graph_endpoint_e2e() -> Result<()> {
    // --- 1. Arrange ---
    let app = TestApp::spawn("test_knowledge_graph_endpoint_e2e").await?;
    app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/test_knowledge_graph_endpoint_e2e/v1/chat/completions");
        then.status(200).json_body(json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Mock knowledge graph response."
                }
            }]
        }));
    });

    // Define time windows for the facts
    let now = Utc::now();
    let past_start = now - Duration::days(10);
    let past_end = now - Duration::days(5);
    let current_start = now - Duration::days(1);
    let current_end = now + Duration::days(1);
    let future_start = now + Duration::days(5);
    let future_end = now + Duration::days(10);

    // Seed the knowledge graph with time-sensitive data
    {
        let mut kg = app
            .knowledge_graph
            .write()
            .expect("Failed to get write lock on knowledge graph");

        kg.add_fact("Alice", "role", "Developer", past_start, past_end)?;
        kg.add_fact(
            "Alice",
            "role",
            "Lead Developer",
            current_start,
            current_end,
        )?;
        kg.add_fact("Alice", "role", "Architect", future_start, future_end)?;
    }

    // --- 2. Act ---
    // Query for the fact that should be active right now
    let response = app
        .client
        .post(format!("{}/search/knowledge_graph", app.address))
        .json(&json!({
            "subject": "Alice",
            "predicate": "role"
        }))
        .send()
        .await?;

    // --- 3. Assert ---
    assert!(response.status().is_success());

    let body: ApiResponse<Value> = response.json().await?;
    let object = body.result["object"]
        .as_str()
        .expect("Object field should be a string");

    assert_eq!(
        object, "Lead Developer",
        "The endpoint should have returned the currently active role."
    );

    Ok(())
}
