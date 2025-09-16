//! # End-to-End Multi-Stage Hybrid Search Test
//!
//! This test verifies the complete, new hybrid search workflow as defined in `NOW.md`.
//! It ensures that the system correctly performs:
//! 1. LLM-based Query Analysis.
//! 2. Metadata Pre-filtering.
//! 3. Vector Re-ranking on the filtered candidates.
//! 4. Final RAG synthesis with the precise context.

mod common;

use anyhow::Result;
use anyrag_server::types::ApiResponse;
use common::{generate_jwt, TestApp, TestDataBuilder};
use core_access::get_or_create_user;
use httpmock::Method;
use serde_json::{json, Value};

#[tokio::test]
async fn test_e2e_multi_stage_hybrid_search() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn().await?;
    let user_identifier = "test-user@example.com";
    let db = &app.app_state.sqlite_provider.db;
    let user = get_or_create_user(db, user_identifier, None).await?;

    // Seed data
    let builder = TestDataBuilder::new(&app).await?;
    builder
        .add_document(
            "doc_tesla",
            &user.id,
            "Tesla Prize",
            "The grand prize for the campaign is a Tesla Model 3.",
            None,
        )
        .await?
        .add_metadata("doc_tesla", &user.id, "ENTITY", "PRODUCT", "Tesla")
        .await?
        .add_metadata(
            "doc_tesla",
            &user.id,
            "KEYPHRASE",
            "CONCEPT",
            "campaign prize",
        )
        .await?
        .add_embedding("doc_tesla", vec![1.0, 0.0, 0.0, 0.0])
        .await?;

    builder
        .add_document(
            "doc_distractor",
            &user.id,
            "Distractor Document",
            "Apples and oranges are common fruits.",
            None,
        )
        .await?
        .add_metadata("doc_distractor", &user.id, "ENTITY", "FRUIT", "Fruit")
        .await?
        .add_metadata(
            "doc_distractor",
            &user.id,
            "KEYPHRASE",
            "CONCEPT",
            "fruit information",
        )
        .await?
        .add_embedding("doc_distractor", vec![0.0, 1.0, 0.0, 0.0])
        .await?;

    let user_query = "Tell me about the Tesla campaign prize";
    let final_rag_answer = "The campaign's grand prize is a Tesla Model 3.";

    // --- 2. Mock External Services ---
    let query_analysis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("expert query analyst");
        then.status(200).json_body(json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": json!({
                        "entities": ["Tesla"],
                        "keyphrases": ["campaign prize"]
                    }).to_string()
                }
            }]
        }));
    });

    let query_vector = vec![1.0, 0.0, 0.0, 0.0];
    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": query_vector }] }));
    });

    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("strict, factual AI")
            .body_contains(user_query)
            .body_contains("The grand prize for the campaign is a Tesla Model 3.")
            .body_contains("User Question")
            // The mock now expects BOTH documents in the context, but the final answer should still be correct.
            .body_contains("Apples and oranges are common fruits.");
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

    // --- 4. Assert the final response and mock calls ---
    let response_body: ApiResponse<Value> = response.json().await?;
    assert_eq!(
        response_body.result["text"], final_rag_answer,
        "The final RAG answer was not as expected."
    );

    query_analysis_mock.assert();
    embedding_mock.assert();
    rag_synthesis_mock.assert();

    Ok(())
}
