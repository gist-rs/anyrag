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

#[tokio::test]
#[cfg(feature = "graph_db")]
async fn test_kg_provides_more_precise_answer() -> Result<()> {
    // --- 1. Arrange ---
    let app = TestApp::spawn().await?;

    // Helper to seed the regular KB with a plausible but less precise answer
    async fn seed_regular_kb(app: &TestApp, answer: &str) -> Result<()> {
        let db = turso::Builder::new_local(app.db_path.to_str().unwrap())
            .build()
            .await?;
        let conn = db.connect()?;
        anyrag::ingest::knowledge::create_kb_tables_if_not_exists(&conn).await?;
        conn.execute(
            "INSERT INTO faq_kb (question, answer, source_url, is_explicit, content_hash, last_modified) VALUES (?, ?, ?, ?, ?, ?)",
            turso::params![
                "What is the power source for the SuperWidget X500?",
                answer,
                "manual_seed",
                true,
                "generic_hash",
                chrono::Utc::now().to_rfc3339()
            ],
        ).await?;
        Ok(())
    }

    let subject = "SuperWidget_X500";
    let query_full_question = "What is the power source for the SuperWidget X500?";
    let generic_answer = "It uses a standard rechargeable battery pack";
    let precise_kg_answer = "The primary power source is the TX300 Solar Array";
    let final_answer_without_kg =
        "The SuperWidget X500 is powered by a standard rechargeable battery pack.";
    let final_answer_with_kg = "The primary power source is the TX300 Solar Array.";

    // Seed the regular KB with the generic answer.
    seed_regular_kb(&app, generic_answer).await?;

    // Seed the Knowledge Graph with the precise, correct answer.
    {
        let mut kg = app.knowledge_graph.write().unwrap();
        kg.add_fact(
            subject,
            "role", // Using "role" as the predicate for consistency
            precise_kg_answer,
            Utc::now() - Duration::days(1),
            Utc::now() + Duration::days(1),
        )?;
    }

    // --- 2. Mock Services ---
    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.5, 0.5, 0.5] }] }));
    });

    // Mock for the RAG call WITHOUT the KG context.
    let rag_without_kg_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("strict, factual AI")
            .body_contains(generic_answer)
            .matches(|req| !String::from_utf8_lossy(req.body.as_deref().unwrap_or_default()).contains("Definitive Answer"));
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_answer_without_kg}}]}),
        );
    });

    // Mock for the RAG call WITH the KG context.
    let rag_with_kg_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("strict, factual AI")
            .body_contains(precise_kg_answer)
            .body_contains(generic_answer); // It should see both contexts
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_answer_with_kg}}]}),
        );
    });

    // --- 3. Act & Assert (Without Knowledge Graph) ---
    let response_without_kg = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .json(&json!({
            "query": query_full_question, // Use the full question here
            "use_knowledge_graph": false
        }))
        .send()
        .await?;

    assert!(response_without_kg.status().is_success());
    let body_without_kg: ApiResponse<Value> = response_without_kg.json().await?;
    assert_eq!(body_without_kg.result["text"], final_answer_without_kg);
    rag_without_kg_mock.assert();

    // --- 4. Act & Assert (With Knowledge Graph) ---
    let response_with_kg = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .json(&json!({
            "query": subject, // Use the clean subject here to match the KG
            "use_knowledge_graph": true
        }))
        .send()
        .await?;

    assert!(response_with_kg.status().is_success());
    let body_with_kg: ApiResponse<Value> = response_with_kg.json().await?;
    assert_eq!(body_with_kg.result["text"], final_answer_with_kg);
    rag_with_kg_mock.assert();

    // Embedding mock should be hit twice, once for each search.
    embedding_mock.assert_hits(2);

    Ok(())
}
