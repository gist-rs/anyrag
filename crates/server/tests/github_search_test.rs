//! # GitHub Multi-Repository Search E2E Test
//!
//! This file contains an end-to-end test for the `POST /search/examples`
//! endpoint, verifying its ability to perform a hybrid search across multiple,
//! versioned, repository-specific databases.

mod common;

use anyhow::Result;
use anyrag_github::ingest::storage::StorageManager;
use anyrag_server::types::ApiResponse;
use common::{generate_jwt, TestApp};
use httpmock::Method;
use serde_json::{json, Value};
use std::fs;
use std::process::Command;
use tempfile::tempdir;
use turso::{params, Builder};

/// Helper function to create and populate a bare Git repository for testing.
fn create_mock_git_repo(
    name: &str,
    files: &[(&str, &str)],
    tag: &str,
) -> Result<tempfile::TempDir> {
    let remote_repo_dir = tempdir()?;
    let remote_repo_path = remote_repo_dir.path();

    Command::new("git")
        .arg("init")
        .arg("--bare")
        .current_dir(remote_repo_path)
        .status()?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow::anyhow!("Failed to init bare repo for {name}"))?;

    let local_repo_dir = tempdir()?;
    let local_repo_path = local_repo_dir.path();

    Command::new("git")
        .arg("clone")
        .arg(remote_repo_path.to_str().unwrap())
        .arg(".")
        .current_dir(local_repo_path)
        .status()?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow::anyhow!("Failed to clone for {name}"))?;

    for (file_name, content) in files {
        let file_path = local_repo_path.join(file_name);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(file_path, *content)?;
    }

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(local_repo_path)
        .status()?;
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(local_repo_path)
        .status()?;
    Command::new("git")
        .arg("add")
        .arg(".")
        .current_dir(local_repo_path)
        .status()?;
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(local_repo_path)
        .status()?;
    Command::new("git")
        .arg("tag")
        .arg(tag)
        .current_dir(local_repo_path)
        .status()?;
    Command::new("git")
        .args(["push", "--tags", "origin", "master"])
        .current_dir(local_repo_path)
        .status()?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow::anyhow!("Failed to push for {name}"))?;

    Ok(remote_repo_dir)
}

#[tokio::test]
async fn test_search_across_multiple_repos_e2e() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn().await?;
    let user_identifier = "github-search-user@example.com";
    let token = generate_jwt(user_identifier)?;

    // A. Mock the embedding API for the ingestion process. It will be called for each repo.
    let mut ingest_embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.1, 0.2, 0.3, 0.4] }] }));
    });

    // B. Create two distinct mock repositories.
    let repo_a_files = [(
        "src/db.rs",
        "/// ```rust\n/// let conn = connect_to_database();\n/// ```\nfn db_logic() {}",
    )];
    let repo_a_dir = create_mock_git_repo("repo-a", &repo_a_files, "v1.0.0")?;
    let repo_a_url = repo_a_dir.path().to_str().unwrap();
    let repo_a_name = StorageManager::url_to_repo_name(repo_a_url);

    let repo_b_files = [("examples/client.rs", "fn make_http_request() {}")];
    let repo_b_dir = create_mock_git_repo("repo-b", &repo_b_files, "v1.2.0")?;
    let repo_b_url = repo_b_dir.path().to_str().unwrap();
    let repo_b_name = StorageManager::url_to_repo_name(repo_b_url);

    // C. Ingest both repositories.
    for (url, version) in [(repo_a_url, "v1.0.0"), (repo_b_url, "v1.2.0")] {
        app.client
            .post(format!("{}/ingest/github", app.address))
            .bearer_auth(token.clone())
            .json(&json!({ "url": url, "version": version }))
            .send()
            .await?
            .error_for_status()?;
    }
    // Assert that ingestion called the embedding service twice.
    ingest_embedding_mock.assert_hits(2);
    // The mock is no longer needed for the ingestion part.
    ingest_embedding_mock.delete();

    // D. Manually update embeddings for the ingested examples to have known vectors.
    let db_a_path = format!("{}/{repo_a_name}.db", anyrag::constants::GITHUB_DB_DIR);
    let db_b_path = format!("{}/{repo_b_name}.db", anyrag::constants::GITHUB_DB_DIR);
    let db_a = Builder::new_local(&db_a_path).build().await?;
    let db_b = Builder::new_local(&db_b_path).build().await?;
    let (conn_a, conn_b) = (db_a.connect()?, db_b.connect()?);

    let vector_a = [1.0, 0.0, 0.0]; // "database" vector
    let vector_b = [0.0, 1.0, 0.0]; // "http" vector
    let vector_a_bytes: &[u8] =
        unsafe { std::slice::from_raw_parts(vector_a.as_ptr() as *const u8, vector_a.len() * 4) };
    let vector_b_bytes: &[u8] =
        unsafe { std::slice::from_raw_parts(vector_b.as_ptr() as *const u8, vector_b.len() * 4) };

    conn_a
        .execute(
            "UPDATE example_embeddings SET embedding = ?",
            params![vector_a_bytes],
        )
        .await?;
    conn_b
        .execute(
            "UPDATE example_embeddings SET embedding = ?",
            params![vector_b_bytes],
        )
        .await?;

    // E. Mock the embedding API for the search query.
    let query_vector = vec![0.99, 0.01, 0.0];
    let search_embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": query_vector }] }));
    });

    // F. Mock query analysis for hybrid search.
    // The handler uses the "query_analysis" task from config.yml, but the underlying
    // github::search_examples function uses its own specific system prompt. We must match that one.
    let query_analysis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            // This substring is unique to the GITHUB_EXAMPLE_SEARCH_ANALYSIS_SYSTEM_PROMPT
            .body_contains("expert code search analyst");
        then.status(200).json_body(json!({
            "choices": [{"message": {"role": "assistant", "content": json!({
                "entities": ["database"],
                "keyphrases": ["connect"]
            }).to_string()}}]
        }));
    });

    // --- 2. Act ---
    let response = app
        .client
        .post(format!("{}/search/examples", app.address))
        .bearer_auth(token)
        .json(&json!({
            "query": "how to connect to a database",
            "repos": [
                format!("{}:v1.0.0", repo_a_name),
                format!("{}:v1.2.0", repo_b_name)
            ]
        }))
        .send()
        .await?
        .error_for_status()?;

    // --- 3. Assert ---
    search_embedding_mock.assert();
    query_analysis_mock.assert();
    let body: ApiResponse<Value> = response.json().await?;
    let results = body.result["results"]
        .as_array()
        .expect("Results should be an array");

    assert_eq!(
        results.len(),
        1,
        "Expected exactly one result from the search"
    );
    let top_result = &results[0];
    assert!(
        top_result["description"]
            .as_str()
            .unwrap()
            .contains("connect_to_database"),
        "The result should be the database example"
    );
    assert!(
        top_result["link"].as_str().unwrap().contains("src/db.rs"),
        "The result link should point to the correct file"
    );

    Ok(())
}
