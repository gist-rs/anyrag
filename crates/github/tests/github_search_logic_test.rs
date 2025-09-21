//! # GitHub Search Logic Test
//!
//! This file contains a focused integration test to verify the correctness of the
//! new metadata-based pre-filtering logic in the GitHub example search.

// Make the common module available.
mod common;

use anyrag_github::{
    ingest::{
        storage::StorageManager,
        types::{ExampleSourceType, GeneratedExample},
    },
    search_examples,
};
use common::{setup_mock_embedding_server, setup_tracing, MockAiProvider};
use serde_json::json;
use std::sync::Arc;
use tempfile::{tempdir, TempDir};

/// Helper function to set up a temporary database with two distinct examples.
/// One example is about "turso", and the other is unrelated.
async fn setup_database_with_mock_data() -> (StorageManager, String, TempDir) {
    // 1. Create a temporary directory for the databases.
    let db_dir = tempdir().expect("Failed to create db temp dir");
    let db_path_str = db_dir.path().to_str().unwrap();

    // 2. Initialize the storage manager.
    let storage = StorageManager::new(Some(db_path_str))
        .await
        .expect("Failed to create StorageManager");

    // 3. Track a mock repository.
    let repo_url = "http://mock.com/user/test-repo";
    let tracked_repo = storage
        .track_repository(repo_url)
        .await
        .expect("Failed to track repo");

    // 4. Create the examples to be stored.
    let version = "v1.0.0";
    let turso_example = GeneratedExample {
        example_handle: "test:tests/turso_test.rs:connect".to_string(),
        content: "let db = Client::open(\"file:local.db\"); // turso client".to_string(),
        source_file: "tests/turso_test.rs".to_string(),
        source_type: ExampleSourceType::Test,
        version: version.to_string(),
    };
    let other_example = GeneratedExample {
        example_handle: "test:tests/other_test.rs:run".to_string(),
        content: "let x = 1 + 1;".to_string(),
        source_file: "tests/other_test.rs".to_string(),
        source_type: ExampleSourceType::Test,
        version: version.to_string(),
    };
    let examples = vec![turso_example, other_example];

    // 5. Store the examples.
    storage
        .store_examples(&tracked_repo, examples)
        .await
        .expect("Failed to store examples");

    // This test focuses on pre-filtering, so we don't need real embeddings.
    // The embedding and storing part would happen in a real `run_github_ingestion` call,
    // but we can skip it here to keep the test focused.

    (storage, tracked_repo.repo_name, db_dir)
}

#[tokio::test]
async fn test_github_search_with_metadata_prefiltering() {
    // --- 1. Arrange ---
    setup_tracing();
    let (storage_manager, repo_name, _db_dir) = setup_database_with_mock_data().await;
    let mock_embedding_server = setup_mock_embedding_server().await;
    let embedding_api_url = format!("{}/v1/embeddings", mock_embedding_server.uri());
    let user_query = "how to connect to turso";

    // Mock the AI provider to return a specific entity from query analysis.
    let mock_ai_responses = vec![
        // This is the response for the `analyze_query` call.
        json!({
            "entities": ["turso"],
            "keyphrases": ["connect"]
        })
        .to_string(),
    ];
    let mock_ai_provider = MockAiProvider::new(mock_ai_responses);
    let call_history = mock_ai_provider.call_history.clone();
    let ai_provider = Arc::new(mock_ai_provider);

    // --- 2. Act ---
    // Execute the search. This should trigger the new pre-filtering logic.
    let search_results = search_examples(
        &storage_manager,
        user_query,
        &[repo_name],
        ai_provider,
        &embedding_api_url,
        "mock-model",
        Some("test_api_key"),
    )
    .await
    .expect("Search failed");

    // --- 3. Assert ---
    // The pre-filtering on the "turso" entity should mean only one result is returned.
    assert_eq!(
        search_results.len(),
        1,
        "Expected exactly one search result after pre-filtering."
    );

    let result = &search_results[0];
    assert!(
        result.description.contains("turso"),
        "The returned result should be the turso example."
    );
    assert_eq!(
        result.title, "test:tests/turso_test.rs:connect",
        "The wrong example was returned."
    );

    // Verify that the query analysis was called as expected.
    let history = call_history.read().unwrap();
    assert_eq!(
        history.len(),
        1,
        "Expected one call to the AI provider for query analysis"
    );

    let (_system_prompt, user_prompt) = &history[0];
    assert!(
        user_prompt.contains(user_query),
        "The user prompt for analysis did not contain the original query."
    );
}
