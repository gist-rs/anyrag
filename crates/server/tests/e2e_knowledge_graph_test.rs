//! # Knowledge Graph E2E Test
//!
//! This test verifies that the `POST /search/knowledge_graph` endpoint correctly
//! queries the in-memory knowledge graph and returns time-sensitive facts.

mod common;

use crate::common::main::types::ApiResponse;
use anyhow::Result;
use chrono::{Duration, Utc};
use common::TestApp;
use httpmock::Method;
use serde_json::{json, Value};

#[tokio::test]
#[cfg(feature = "graph_db")]
async fn test_knowledge_graph_endpoint_e2e() -> Result<()> {
    // --- 1. Arrange ---
    let app = TestApp::spawn().await?;

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
        // The lock is scoped to ensure it's released before the API call
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

#[tokio::test]
#[cfg(feature = "graph_db")]
async fn test_hybrid_search_with_knowledge_graph_context() -> Result<()> {
    // --- 1. Arrange ---
    let app = TestApp::spawn().await?;

    // The knowledge search handler queries the `faq_kb` table. We must ensure it exists.
    let db = turso::Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;
    anyrag::ingest::knowledge::create_kb_tables_if_not_exists(&conn).await?;

    // Define a unique fact to seed the knowledge graph.
    let now = Utc::now();
    let start_time = now - Duration::days(1);
    let end_time = now + Duration::days(1);
    let unique_subject = "SuperWidget X500";
    let unique_predicate = "role";
    let unique_object = "The primary power source";

    {
        let mut kg = app
            .knowledge_graph
            .write()
            .expect("Failed to get write lock on KG");
        kg.add_fact(
            unique_subject,
            unique_predicate,
            unique_object,
            start_time,
            end_time,
        )?;
    }

    // Mock the embedding service for the initial search query.
    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/embeddings")
            .body_contains(unique_subject);
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.1, 0.2, 0.3] }] }));
    });

    // Mock the final RAG synthesis LLM call.
    // The key assertion is that the request body MUST contain our unique fact.
    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            // Distinguish this from other AI calls by checking for the RAG system prompt.
            .body_contains("strict, factual AI")
            // Check that the context contains the prepended fact from the KG.
            .body_matches(
                regex::Regex::new(&format!(
                    "(?s)Definitive Answer from Knowledge Graph: {}\\.",
                    regex::escape(unique_object)
                ))
                .unwrap(),
            );
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": "Acknowledged."}}]}),
        );
    });

    // --- 2. Act ---
    // Call the main RAG endpoint with the special flag enabled.
    let response = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .json(&json!({
            "query": unique_subject,
            "use_knowledge_graph": true
        }))
        .send()
        .await?;

    // --- 3. Assert ---
    assert!(response.status().is_success(), "The API call failed.");

    // This is the most important assertion. It verifies that the LLM was called
    // with the correctly augmented context. If it wasn't, the mock would not
    // have been hit, and this would panic.
    embedding_mock.assert();
    rag_synthesis_mock.assert();

    Ok(())
}
