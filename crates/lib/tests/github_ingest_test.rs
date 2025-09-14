//! # GitHub Ingestion Pipeline Integration Test
//!
//! This test verifies the core logic of the ingestion pipeline, focusing on the
//! interaction between the Extractor and the StorageManager without a real Git remote.

use anyrag::github_ingest::{
    extractor::Extractor, storage::StorageManager, types::ExampleSourceType,
};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

/// Helper to create a file with content in a directory. Panics on failure.
fn create_file(path: &Path, name: &str, content: &str) {
    let file_path = path.join(name);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).expect("Failed to create parent directory for test file");
    }
    fs::write(file_path, content).expect("Failed to write test file");
}

#[tokio::test]
async fn test_extractor_and_storage_integration() {
    // --- 1. Arrange ---
    // Create a mock directory representing a cloned repository.
    let repo_dir = tempdir().expect("Failed to create repo temp dir");
    let repo_path = repo_dir.path();

    // Create a separate directory for the databases.
    let db_dir = tempdir().expect("Failed to create db temp dir");
    let db_path_str = db_dir.path().to_str().unwrap();

    let common_code = "println!(\"hello world\");";
    let version = "v1.0.0";

    // Create a set of mock source files with examples. The `common_code` is
    // duplicated across sources with different priorities to test conflict resolution.
    create_file(
        repo_path,
        "README.md",
        &format!("```rust\n{common_code}\n```"),
    );
    create_file(
        repo_path,
        "examples/main.rs",
        "fn main() { println!(\"from example\"); }",
    );
    create_file(
        repo_path,
        "src/lib.rs",
        &format!("/// ```rust\n/// {common_code}\n/// ```\nfn lib_fn(){{}}"),
    );
    // This is the highest priority source for the common_code.
    create_file(
        repo_path,
        "tests/test.rs",
        &format!("#[test]\nfn a_test() {{ {common_code}\n}}"),
    );

    // --- 2. Act ---
    // Run the extraction process. This includes conflict resolution.
    let final_examples = Extractor::extract(repo_path, version).unwrap();

    // Initialize the storage manager and create the repository database.
    let storage = StorageManager::new(db_path_str).await.unwrap();
    let repo_url = "http://mock.com/user/test-repo";
    let repo_name = StorageManager::url_to_repo_name(repo_url);
    let tracked_repo = storage.track_repository(repo_url).await.unwrap();

    // Store the extracted examples in the newly created database.
    let store_result = storage.store_examples(&tracked_repo, final_examples).await;

    // --- 3. Assert ---
    // Assert that storage was successful and stored the correct number of examples.
    assert!(
        store_result.is_ok(),
        "store_examples failed: {:?}",
        store_result.err()
    );
    assert_eq!(
        store_result.unwrap(),
        2,
        "Expected to store 2 examples after conflict resolution."
    );

    // Retrieve the examples from the database to verify their content and priority.
    let retrieved_examples = storage.get_examples(&repo_name, version).await.unwrap();
    assert_eq!(
        retrieved_examples.len(),
        2,
        "Expected 2 examples to be retrieved from the database."
    );

    // Sort results by source type to make assertions deterministic.
    let mut sorted_retrieved = retrieved_examples;
    sorted_retrieved.sort_by_key(|ex| ex.source_type);

    // The lowest priority example should be from the example file.
    let file_example = &sorted_retrieved[0];
    assert_eq!(file_example.source_type, ExampleSourceType::ExampleFile);
    assert_eq!(
        file_example.content.trim(),
        "fn main() { println!(\"from example\"); }"
    );
    assert_eq!(file_example.source_file, "examples/main.rs");

    // The highest priority example should be the one from the test file.
    let test_example = &sorted_retrieved[1];
    assert_eq!(test_example.source_type, ExampleSourceType::Test);
    assert_eq!(test_example.content.trim(), common_code);
    assert_eq!(test_example.source_file, "tests/test.rs");
}
