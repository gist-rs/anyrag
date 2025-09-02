//! # Knowledge Search Logic Test
//!
//! This file contains a focused integration test to verify the correctness of the
//! multi-stage hybrid search logic, isolated from the web server.

mod common;

use anyhow::Result;
use anyrag::{
    providers::db::sqlite::SqliteProvider,
    search::hybrid_search,
    types::{ContentType, ExecutePromptOptions, PromptClientBuilder},
};
use common::{setup_tracing, MockAiProvider};
use core_access::GUEST_USER_IDENTIFIER;
use serde_json::json;
use turso::params;
use uuid::Uuid;

/// A helper function to set up an in-memory database with manually inserted,
/// distinct vectors, documents, and metadata, all owned by the guest user.
async fn setup_database_with_manual_data() -> Result<SqliteProvider> {
    // 1. Create a new, isolated in-memory database and initialize schema.
    let provider = SqliteProvider::new(":memory:").await?;
    provider.initialize_schema().await?;
    let conn = provider
        .db
        .connect()
        .expect("Failed to get connection for test setup");

    // --- Calculate the deterministic Guest User ID ---
    let guest_user_id =
        Uuid::new_v5(&Uuid::NAMESPACE_URL, GUEST_USER_IDENTIFIER.as_bytes()).to_string();

    // --- Document 1: Tesla (owned by Guest) ---
    let doc1_id = "doc_tesla";
    let doc1_vector: Vec<f32> = vec![1.0, 0.0, 0.0, 0.0];
    let doc1_vector_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(doc1_vector.as_ptr() as *const u8, doc1_vector.len() * 4)
    };
    conn.execute(
        "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?, ?, ?, ?, ?)",
        params![
            doc1_id,
            guest_user_id.clone(),
            "http://mock.com/tesla",
            "Tesla Prize",
            "The grand prize is a Tesla."
        ],
    )
    .await?;
    conn.execute(
        "INSERT INTO content_metadata (document_id, owner_id, metadata_type, metadata_value) VALUES (?, ?, ?, ?)",
        params![doc1_id, guest_user_id.clone(), "ENTITY", "Tesla"],
    )
    .await?;
    conn.execute(
        "INSERT INTO document_embeddings (document_id, model_name, embedding) VALUES (?, ?, ?)",
        params![doc1_id, "mock-model", doc1_vector_bytes],
    )
    .await?;

    // --- Document 2: Unrelated (owned by Guest) ---
    let doc2_id = "doc_unrelated";
    let doc2_vector: Vec<f32> = vec![0.0, 1.0, 0.0, 0.0];
    let doc2_vector_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(doc2_vector.as_ptr() as *const u8, doc2_vector.len() * 4)
    };
    conn.execute(
        "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?, ?, ?, ?, ?)",
        params![
            doc2_id,
            guest_user_id.clone(),
            "http://mock.com/other",
            "Other Info",
            "This is another document."
        ],
    )
    .await?;
    conn.execute(
        "INSERT INTO content_metadata (document_id, owner_id, metadata_type, metadata_value) VALUES (?, ?, ?, ?)",
        params![doc2_id, guest_user_id.clone(), "ENTITY", "Other"],
    )
    .await?;
    conn.execute(
        "INSERT INTO document_embeddings (document_id, model_name, embedding) VALUES (?, ?, ?)",
        params![doc2_id, "mock-model", doc2_vector_bytes],
    )
    .await?;

    Ok(provider)
}

/// This test verifies the core logic of the multi-stage hybrid search.
///
/// It uses a manually prepared database and mocks the AI provider to ensure that:
/// 1. The query analysis correctly extracts an entity.
/// 2. The metadata search uses that entity to find the correct document ID.
/// 3. The vector search is restricted to that document ID and returns the correct document.
/// 4. The final RAG context contains *only* the correct document's content.
#[tokio::test]
async fn test_hybrid_search_logic_is_correct() -> Result<()> {
    // --- Arrange ---
    setup_tracing();
    let sqlite_provider = setup_database_with_manual_data()
        .await
        .expect("Database setup failed");

    let user_query = "Tell me about the Tesla prize";
    let query_vector = vec![0.99, 0.01, 0.0, 0.0]; // Vector is very close to the Tesla doc

    // Mock the AI provider for two separate calls
    let mock_ai_responses = vec![
        // 1. Response for Query Analysis
        json!({
            "entities": ["Tesla"],
            "keyphrases": ["prize"]
        })
        .to_string(),
        // 2. Response for final RAG Synthesis
        "The grand prize is indeed a Tesla.".to_string(),
    ];
    let mock_ai_provider = MockAiProvider::new(mock_ai_responses);
    let call_history = mock_ai_provider.call_history.clone();

    // --- Act ---
    // This section manually replicates the logic from the server's knowledge_search_handler
    let limit = 5;
    // Call with `owner_id: None` to simulate a guest user request.
    let search_results = hybrid_search(
        &sqlite_provider,
        &mock_ai_provider,
        query_vector,
        user_query,
        None, // owner_id
        limit,
        "You are an expert query analyst.",
        "USER QUERY:\n{prompt}",
    )
    .await?;

    // --- Assert Pre-computation ---
    assert_eq!(
        search_results.len(),
        1,
        "Hybrid search should have returned exactly one result."
    );
    assert_eq!(
        search_results[0].title, "Tesla Prize",
        "The wrong document was returned by hybrid search."
    );

    // --- Act (RAG Synthesis) ---
    let context = search_results[0].description.clone();

    let client = PromptClientBuilder::new()
        .ai_provider(Box::new(mock_ai_provider))
        .storage_provider(Box::new(sqlite_provider))
        .build()?;

    let options = ExecutePromptOptions {
        prompt: user_query.to_string(),
        content_type: Some(ContentType::Knowledge),
        context: Some(context),
        ..Default::default()
    };
    let final_result = client.execute_prompt_with_options(options).await?;

    // --- Assert Final Result and AI Calls ---
    assert_eq!(final_result.text, "The grand prize is indeed a Tesla.");

    let history = call_history.read().unwrap();
    assert_eq!(history.len(), 2, "Expected two calls to the AI provider");

    // Assert Query Analysis call
    let (system_prompt_1, user_prompt_1) = &history[0];
    assert!(system_prompt_1.contains("expert query analyst"));
    assert_eq!(*user_prompt_1, format!("USER QUERY:\n{user_query}"));

    // Assert RAG Synthesis call (most important assertion)
    let (system_prompt_2, user_prompt_2) = &history[1];
    assert!(system_prompt_2.contains("strict, factual AI"));
    assert!(user_prompt_2.contains("The grand prize is a Tesla."));
    assert!(!user_prompt_2.contains("This is another document."));

    Ok(())
}
