//! # Temporal Reasoning E2E Test
//!
//! This test file verifies that the RAG pipeline can correctly answer questions
//! that require temporal reasoning, such as finding the "newest" or "latest" item.

mod common;

use anyhow::Result;
use anyrag_server::types::ApiResponse;
use common::{generate_jwt, TestApp, TestDataBuilder};
use core_access::get_or_create_user;
use httpmock::Method;
use serde_json::{json, Value};

#[tokio::test]
async fn test_temporal_query_for_newest_item() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn().await?;
    let user_identifier = "temporal-user@example.com";
    let db = &app.app_state.sqlite_provider.db;
    let user = get_or_create_user(db, user_identifier, None).await?;

    // Seed the database with time-stamped facts and a distractor document.
    let builder = TestDataBuilder::new(&app).await?;

    // iPhone 16 (older)
    builder
        .add_document(
            "doc_iphone_16",
            &user.id,
            "iPhone 16",
            "The iPhone 16, released in 2024, features the A18 chip.",
            None,
        )
        .await?
        .add_metadata("doc_iphone_16", &user.id, "ENTITY", "PRODUCT", "iPhone")
        .await?
        .add_metadata(
            "doc_iphone_16",
            &user.id,
            "PROPERTY",
            "release_date",
            "2024-09-09",
        )
        .await?
        .add_embedding("doc_iphone_16", vec![0.8, 0.1, 0.1])
        .await?;

    // iPhone 17 (newer)
    builder
        .add_document(
            "doc_iphone_17",
            &user.id,
            "iPhone 17",
            "The iPhone 17, released in 2025, features the A19 chip.",
            None,
        )
        .await?
        .add_metadata("doc_iphone_17", &user.id, "ENTITY", "PRODUCT", "iPhone")
        .await?
        .add_metadata(
            "doc_iphone_17",
            &user.id,
            "PROPERTY",
            "release_date",
            "2025-09-09",
        )
        .await?
        .add_embedding("doc_iphone_17", vec![0.9, 0.1, 0.1])
        .await?;

    // Alice's phone (distractor, related to iPhone 16)
    builder
        .add_document(
            "doc_alice_phone",
            &user.id,
            "Alice's Phone",
            "Alice has an iPhone 16.",
            None,
        )
        .await?
        .add_metadata("doc_alice_phone", &user.id, "ENTITY", "PERSON", "Alice")
        .await?;

    let user_query = "Bob wants the newest iPhone, what should he get?";
    let final_rag_answer =
        "Based on the release date, Bob should get the iPhone 17, which features the A19 chip.";

    // --- 2. Mock External Services ---
    let query_analysis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .body_contains("expert query analyst");
        then.status(200).json_body(json!({
            "choices": [{"message": {"role": "assistant", "content": json!({
                "entities": ["iPhone"],
                "keyphrases": ["newest"] // The keyword that should trigger the temporal logic
            }).to_string()}}]
        }));
    });

    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [1.0, 0.0, 0.0] }] }));
    });

    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .body_contains("strict, factual AI")
            // CRUCIAL: It must have the context for the iPhone 17.
            .body_contains("The iPhone 17, released in 2025")
            // CRUCIAL: It MUST NOT have the context for the older iPhone 16.
            .matches(|req| {
                !String::from_utf8_lossy(req.body.as_deref().unwrap_or_default())
                    .contains("iPhone 16")
            });
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_rag_answer}}]}),
        );
    });

    // --- 3. Execute the search ---
    let token = generate_jwt(user_identifier)?;
    let response = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .bearer_auth(token)
        .json(&json!({ "query": user_query }))
        .send()
        .await?
        .error_for_status()?;

    // --- 4. Assert ---
    let response_body: ApiResponse<Value> = response.json().await?;
    assert_eq!(response_body.result["text"], final_rag_answer);

    query_analysis_mock.assert();
    embedding_mock.assert();
    rag_synthesis_mock.assert();

    Ok(())
}
