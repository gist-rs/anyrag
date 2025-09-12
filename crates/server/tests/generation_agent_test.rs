//! # Generation Agent E2E Test
//!
//! This file contains an end-to-end test for the refactored `/gen/text` endpoint,
//! verifying that its new agentic, tool-selecting logic works as intended.

mod common;

use anyhow::Result;
use anyrag_server::types::ApiResponse;
use core_access::get_or_create_user;
use httpmock::Method;
use serde_json::{json, Value};
use turso::{params, Database};

use crate::common::{generate_jwt, TestApp};

/// Seeds the database using the exact same database object from the running TestApp's state.
async fn seed_data(db: &Database, user_identifier: &str) -> Result<()> {
    // This connection comes from the same pool the server is using.
    let conn = db.connect()?;
    let user = get_or_create_user(db, user_identifier, None).await?;

    // --- Seed Data ---
    let doc1_id = "doc_love";
    let doc1_vector: Vec<f32> = vec![1.0, 0.0, 0.0];
    let doc1_vector_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(doc1_vector.as_ptr() as *const u8, doc1_vector.len() * 4)
    };
    conn.execute(
        "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?, ?, ?, ?, ?)",
        params![
            doc1_id,
            user.id.clone(),
            "http://m.com/love",
            "A Story of Love",
            "This story is about a heartwarming romance."
        ],
    )
    .await?;
    conn.execute(
        "INSERT INTO content_metadata (document_id, owner_id, metadata_type, metadata_subtype, metadata_value) VALUES (?, ?, ?, ?, ?)",
        params![doc1_id, user.id.clone(), "KEYPHRASE", "CONCEPT", "love stories"],
    )
    .await?;
    conn.execute(
        "INSERT INTO document_embeddings (document_id, model_name, embedding) VALUES (?, ?, ?)",
        params![doc1_id, "mock-embedding-model", doc1_vector_bytes],
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_gen_text_agent_chooses_knowledge_search() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn().await?;
    let user_identifier = "agent-test-user@example.com";

    // Seed data using the server's own database connection pool.
    seed_data(&app.app_state.sqlite_provider.db, user_identifier).await?;

    let context_prompt = "Find the best story about betrayal and forgiveness.";
    let final_generation = "Generated post about a heartwarming romance.";

    // --- 2. Mock External Services ---
    let deconstruction_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("You are a query analyst");
        then.status(200).json_body(json!({
            "choices": [{"message": {"role": "assistant", "content": json!({
                "search_query": "the best love stories",
                "generative_intent": "Find the best story about betrayal and forgiveness."
            }).to_string()}}]
        }));
    });

    let agent_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("intelligent agent that analyzes a user's request");
        then.status(200).json_body(json!({
            "choices": [{"message": {"role": "assistant", "content": json!({
                "tool": "knowledge_search",
                "query": "the best love stories"
            }).to_string()}}]
        }));
    });

    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.9, 0.1, 0.0] }] }));
    });

    let query_analysis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("expert query analyst");
        then.status(200).json_body(json!({
            "choices": [{"message": {"role": "assistant", "content": json!({
                "entities": [],
                "keyphrases": ["love stories"]
            }).to_string()}}]
        }));
    });

    let final_generation_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("heartwarming romance");
        then.status(200).json_body(json!({
            "choices": [{"message": {"role": "assistant", "content": final_generation}}]
        }));
    });

    // --- 3. Execute the /gen/text request ---
    let token = generate_jwt(user_identifier)?;
    let payload = json!({
        "context_prompt": context_prompt,
        "generation_prompt": "Write a Pantip-style post about a heartwarming romance."
    });

    let response = app
        .client
        .post(format!("{}/gen/text", app.address))
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await?
        .error_for_status()?;

    // --- 4. Assert the Final Response and Mock Calls ---
    let body: ApiResponse<Value> = response.json().await?;
    assert_eq!(body.result["text"], final_generation);

    deconstruction_mock.assert();
    agent_mock.assert();
    embedding_mock.assert();
    query_analysis_mock.assert();

    final_generation_mock.assert();

    Ok(())
}
