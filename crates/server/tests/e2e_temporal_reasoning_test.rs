//! # Temporal Reasoning E2E Test
//!
//! This test verifies the temporal reasoning feature of the `POST /search/knowledge`
//! endpoint. It ensures that when a query contains a temporal keyword (e.g., "newest"),
//! the RAG pipeline correctly identifies and prioritizes the most recent document
//! based on a specified metadata property (e.g., `release_date`).

mod common;

use anyhow::Result;
use anyrag_server::types::ApiResponse;
use common::{generate_jwt, TestApp, TestDataBuilder};
use core_access::get_or_create_user;
use httpmock::Method;
use serde_json::{json, Value};

#[tokio::test]
async fn test_knowledge_search_with_temporal_reasoning() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn().await?;
    let user_identifier = "temporal-user@example.com";
    let db = &app.app_state.sqlite_provider.db;
    let user = get_or_create_user(db, user_identifier, None).await?;

    let final_rag_answer = "The newest product is the Anyrag Pro.";

    // --- 2. Seed the Database with Time-Sensitive Data ---
    // We create two documents about products, one older and one newer, distinguished
    // by the `release_date` metadata property.
    let builder = TestDataBuilder::new(&app).await?;

    // A. The older document.
    builder
        .add_document(
            "doc_old",
            &user.id,
            "Anyrag Basic",
            r#"sections:
- title: "Anyrag Basic Overview"
  faqs:
    - question: "What is Anyrag Basic?"
      answer: "Anyrag Basic is a standard product."
"#,
            Some("http://mock.com/doc_old"),
        )
        .await?
        .add_metadata("doc_old", &user.id, "KEYPHRASE", "CONCEPT", "product")
        .await?
        .add_metadata(
            "doc_old",
            &user.id,
            "KEYPHRASE",
            "release_date",
            "2024-01-01",
        )
        .await?
        .add_embedding("doc_old", vec![1.0, 0.0, 0.0])
        .await?;

    // B. The newer document.
    builder
        .add_document(
            "doc_new",
            &user.id,
            "Anyrag Pro",
            r#"sections:
- title: "Anyrag Pro Features"
  faqs:
    - question: "What is Anyrag Pro?"
      answer: "The newest product is the Anyrag Pro."
"#,
            Some("http://mock.com/doc_new"),
        )
        .await?
        .add_metadata("doc_new", &user.id, "KEYPHRASE", "CONCEPT", "product")
        .await?
        .add_metadata(
            "doc_new",
            &user.id,
            "KEYPHRASE",
            "release_date",
            "2024-02-01",
        )
        .await?
        // This vector is intentionally less similar to the query vector than the old doc's
        // to prove that temporal ranking overrides vector similarity.
        .add_embedding("doc_new", vec![0.5, 0.5, 0.0])
        .await?;

    // --- 3. Mock External Services ---
    // A. Mock query analysis to extract the temporal keyword and the discoverable keyphrase.
    let query_analysis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("expert query analyst");
        then.status(200).json_body(json!({
            "choices": [{"message": {"role": "assistant", "content": json!({
                "entities": [],
                "keyphrases": ["newest", "product"] // The AI finds both keywords.
            }).to_string()}}]
        }));
    });

    // B. Mock embedding for the search query. The vector is closer to the OLD document.
    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": vec![0.9, 0.1, 0.0] }] }));
    });

    // C. Mock the final RAG synthesis call.
    // This is the most critical part of the test. We assert that the context provided
    // to the final LLM call contains ONLY the content from the NEWEST document.
    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("strict, factual AI") // Match the RAG system prompt
            .body_contains("## Anyrag Pro Features") // Must contain content from the NEW doc
            .matches(|req| {
                // Must NOT contain content from the OLD doc
                !String::from_utf8_lossy(req.body.as_deref().unwrap_or_default())
                    .contains("Anyrag Basic Overview")
            });
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_rag_answer}}]}),
        );
    });

    // --- 4. Act: Perform a RAG search with a temporal keyword ---
    let token = generate_jwt(user_identifier)?;
    let search_response = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .bearer_auth(token)
        .json(&json!({ "query": "What is the newest product?" }))
        .send()
        .await?
        .error_for_status()?;

    // --- 5. Assert the final response and that all mocks were called ---
    let search_body: ApiResponse<Value> = search_response.json().await?;
    assert_eq!(search_body.result["text"], final_rag_answer);

    query_analysis_mock.assert();
    embedding_mock.assert();
    rag_synthesis_mock.assert();

    Ok(())
}
