//! # Knowledge Base Hybrid Search E2E Test
//!
//! This test verifies that the `POST /search/knowledge` endpoint correctly uses
//! a hybrid search strategy (metadata, vector, keyword) and the YAML chunking
//! logic to retrieve context for the RAG pipeline.

mod common;

use anyhow::Result;
use anyrag_server::types::ApiResponse;
use common::{generate_jwt, TestApp, TestDataBuilder};
use core_access::get_or_create_user;
use httpmock::Method;
use serde_json::{json, Value};

#[tokio::test]
async fn test_knowledge_hybrid_search_workflow() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn("test_knowledge_hybrid_search_workflow").await?;
    let user_identifier = "test-user-khs@example.com";
    let db = &app.app_state.sqlite_provider.db;
    let user = get_or_create_user(db, user_identifier, None).await?;

    // --- 2. Define Test Data and Vectors ---
    let faq_keyword_question = "How does the Quantum Widget work?";
    let faq_keyword_answer = "The Quantum Widget operates on principles of quantum entanglement.";
    let faq_keyword_vector = vec![1.0, 0.0, 0.0, 0.0];

    let faq_vector_question = "What is the method for advanced data processing?";
    let faq_vector_answer =
        "The method for advanced data processing involves multi-layered abstraction.";
    let faq_vector_vector = vec![0.0, 1.0, 0.0, 0.0];

    let final_rag_answer = "The Quantum Widget uses quantum entanglement, and advanced data processing uses multi-layered abstraction.";

    // --- 3. Seed the Database with data in the expected YAML format ---
    let builder = TestDataBuilder::new(&app).await?;
    builder
        .add_document(
            "doc_keyword",
            &user.id,
            faq_keyword_question,
            &format!(
                r#"
sections:
  - title: "{faq_keyword_question}"
    faqs:
      - question: "{faq_keyword_question}"
        answer: "{faq_keyword_answer}""#
            ),
            None,
        )
        .await?
        .add_metadata(
            "doc_keyword",
            &user.id,
            "ENTITY",
            "PRODUCT",
            "Quantum Widget",
        )
        .await?
        .add_embedding("doc_keyword", faq_keyword_vector)
        .await?;

    builder
        .add_document(
            "doc_vector",
            &user.id,
            faq_vector_question,
            &format!(
                r#"
sections:
  - title: "{faq_vector_question}"
    faqs:
      - question: "{faq_vector_question}"
        answer: "{faq_vector_answer}""#
            ),
            None,
        )
        .await?
        .add_metadata(
            "doc_vector",
            &user.id,
            "KEYPHRASE",
            "CONCEPT",
            "advanced data processing",
        )
        .await?
        .add_embedding("doc_vector", faq_vector_vector)
        .await?;

    // --- 4. Mock External Services ---
    let query_analysis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/test_knowledge_hybrid_search_workflow/v1/chat/completions")
            .body_contains("expert query analyst");
        then.status(200).json_body(json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": json!({
                        "entities": ["Quantum Widget"],
                        "keyphrases": ["advanced data processing"]
                    }).to_string()
                }
            }]
        }));
    });

    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/test_knowledge_hybrid_search_workflow/v1/embeddings")
            .body_contains("complex Quantum Widget");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": vec![0.5, 0.5, 0.0, 0.0] }] }));
    });

    // This mock must be very specific to be chosen over the generic one in the harness.
    // It matches the RAG system prompt AND the content from the two chunks that hybrid
    // search should retrieve and format.
    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/test_knowledge_hybrid_search_workflow/v1/chat/completions")
            .body_contains("strict, factual AI") // From RAG_SYNTHESIS_SYSTEM_PROMPT
            .body_contains("## How does the Quantum Widget work?") // From the first chunk
            .body_contains("## What is the method for advanced data processing?"); // From the second chunk
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_rag_answer}}]}),
        );
    });

    // --- 5. Execute Hybrid RAG Search and Verify ---
    let token = generate_jwt(user_identifier)?;
    let search_res = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .bearer_auth(token)
        .json(&json!({ "query": "tell me about complex Quantum Widget" }))
        .send()
        .await?
        .error_for_status()?;

    let search_body: ApiResponse<Value> = search_res.json().await?;
    assert_eq!(search_body.result["text"], final_rag_answer);

    // --- 6. Assert Mock Calls ---
    query_analysis_mock.assert();
    embedding_mock.assert();
    rag_synthesis_mock.assert();

    Ok(())
}
