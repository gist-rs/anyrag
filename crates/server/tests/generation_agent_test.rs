//! # Generation Agent E2E Test
//!
//! This file contains an end-to-end test for the refactored `/gen/text` endpoint.
//! It verifies that the generation workflow functions correctly when explicitly
//! directed to use the `knowledge_search` tool, which involves YAML parsing and chunking.

mod common;

use anyhow::Result;
use anyrag_server::types::ApiResponse;
use core_access::get_or_create_user;
use httpmock::Method;
use serde_json::{json, Value};

use crate::common::{generate_jwt, TestApp, TestDataBuilder};

#[tokio::test]
async fn test_gen_text_with_explicit_knowledge_search() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn().await?;
    let user_identifier = "agent-test-user@example.com";
    let db = &app.app_state.sqlite_provider.db;
    let user = get_or_create_user(db, user_identifier, None).await?;

    // Seed data that the knowledge_search tool will find.
    // The content is in the structured YAML format that the search pipeline expects.
    let builder = TestDataBuilder::new(&app).await?;
    builder
        .add_document(
            "doc_love",
            &user.id,
            "A Story of Love",
            r#"
sections:
  - title: "The Story of Love"
    faqs:
      - question: "What is this story about?"
        answer: "This story is about a heartwarming romance."
"#,
            None,
        )
        .await?
        .add_metadata("doc_love", &user.id, "KEYPHRASE", "CONCEPT", "love stories")
        .await?
        .add_embedding("doc_love", vec![1.0, 0.0, 0.0])
        .await?;

    let context_prompt = "Find the best story about betrayal and forgiveness.";
    let final_generation = "Generated post about a heartwarming romance.";

    // --- 2. Mock External Services ---
    // A. Mock the embedding for the knowledge search.
    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.9, 0.1, 0.0] }] }));
    });

    // B. Mock the query analysis required by the hybrid search pipeline.
    let query_analysis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("expert query analyst"); // Unique to QUERY_ANALYSIS_SYSTEM_PROMPT
        then.status(200).json_body(json!({
            "choices": [{"message": {"role": "assistant", "content": json!({
                "entities": [],
                "keyphrases": ["love stories"]
            }).to_string()}}]
        }));
    });

    // C. Mock the final content generation call.
    // This mock needs to be highly specific to be chosen over the default mock.
    // We match on unique parts of both the generation_prompt and the retrieved context,
    // using the correct chained syntax for the httpmock version in use.
    let final_generation_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("Write a Pantip-style post about a heartwarming romance.") // From generation_prompt
            .body_contains("## The Story of Love"); // From the YAML chunk in the context
        then.status(200).json_body(json!({
            "choices": [{"message": {"role": "assistant", "content": final_generation}}]
        }));
    });

    // --- 3. Execute the /gen/text request ---
    let token = generate_jwt(user_identifier)?;
    // Explicitly tell the handler to use knowledge_search, bypassing the agent.
    let payload = json!({
        "context_prompt": context_prompt,
        "generation_prompt": "Write a Pantip-style post about a heartwarming romance.",
        "use_knowledge_search": true
    });

    let response = app
        .client
        .post(format!("{}/gen/text?debug=true", app.address))
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await?
        .error_for_status()?;

    // --- 4. Assert the Final Response and Mock Calls ---
    let body: ApiResponse<Value> = response.json().await?;
    assert_eq!(
        body.result["text"], final_generation,
        "The final generated text did not match the expected output."
    );

    // Verify from the debug info that the correct tool was used.
    let debug_info = body.debug.unwrap();
    let agent_decision = &debug_info["context_retrieval_details"]["agent_decision"];
    assert_eq!(
        agent_decision["tool"], "knowledge_search",
        "The handler did not use the explicitly requested 'knowledge_search' tool."
    );
    let search_results_count =
        debug_info["context_retrieval_details"]["search_results_count"].as_u64();
    assert_eq!(
        search_results_count,
        Some(1),
        "Expected exactly one contextual chunk from the search."
    );

    // Verify the necessary mocks were called.
    embedding_mock.assert();
    query_analysis_mock.assert();
    final_generation_mock.assert();

    Ok(())
}
