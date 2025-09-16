//! # GitHub Ingestion E2E Test
//!
//! This file contains an end-to-end test for the `POST /ingest/github` endpoint.
//! It verifies the entire workflow from cloning a repository to storing the
//! extracted examples in the database.

mod common;

use anyhow::Result;
use anyrag_server::types::ApiResponse;
use common::{generate_jwt, TestApp};
use github::ingest::storage::StorageManager;

use serde_json::{json, Value};
use std::fs;
use std::process::Command;
use tempfile::tempdir;
use turso::{params, Builder};

#[tokio::test]
async fn test_github_ingestion_e2e_workflow() -> Result<()> {
    // --- 1. Arrange & Setup ---
    // Create a temporary directory to act as a bare remote git repository.
    let remote_repo_dir = tempdir()?;
    let remote_repo_path = remote_repo_dir.path();

    // Initialize a bare git repository.
    Command::new("git")
        .arg("init")
        .arg("--bare")
        .current_dir(remote_repo_path)
        .status()?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow::anyhow!("Failed to initialize bare git repository"))?;

    // Create a separate directory for a local clone to add files.
    let local_repo_dir = tempdir()?;
    let local_repo_path = local_repo_dir.path();

    // Clone the bare repo.
    Command::new("git")
        .arg("clone")
        .arg("--no-local")
        .arg("--depth=1")
        .arg(remote_repo_path.to_str().unwrap())
        .arg(".")
        .current_dir(local_repo_path)
        .status()?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow::anyhow!("Failed to clone repository"))?;

    // Create mock files with examples.
    let readme_content = r#"
# Test Repo
Heres an example:
```rust
fn main() {
    println!("Hello, GitHub!");
}
```
"#;
    fs::write(local_repo_path.join("README.md"), readme_content)?;

    let cargo_toml_content = r#"
[package]
name = "test-repo"
version = "0.1.0-test"
"#;
    fs::write(local_repo_path.join("Cargo.toml"), cargo_toml_content)?;

    // Commit and push the files to the "remote" bare repository.
    Command::new("git")
        .arg("config")
        .arg("user.email")
        .arg("test@example.com")
        .current_dir(local_repo_path)
        .status()?;
    Command::new("git")
        .arg("config")
        .arg("user.name")
        .arg("Test User")
        .current_dir(local_repo_path)
        .status()?;
    Command::new("git")
        .arg("add")
        .arg(".")
        .current_dir(local_repo_path)
        .status()?;
    Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg("Initial commit")
        .current_dir(local_repo_path)
        .status()?;
    Command::new("git")
        .arg("push")
        .arg("origin")
        .arg("master") // Using master for consistency in test env
        .current_dir(local_repo_path)
        .status()?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow::anyhow!("Failed to push to remote"))?;

    let app = TestApp::spawn().await?;
    let user_identifier = "github-ingest-user@example.com";
    let token = generate_jwt(user_identifier)?;

    // --- 2. Act ---
    // Send a request to the `/ingest/github` endpoint.
    // The URL is the local path to our bare repository.
    let remote_url_str = remote_repo_path.to_str().unwrap();
    let response = app
        .client
        .post(format!("{}/ingest/github", app.address))
        .bearer_auth(token)
        .json(&json!({
            "url": remote_url_str,
            // No version is specified, so it should fall back to Cargo.toml
        }))
        .send()
        .await?
        .error_for_status()?;

    // --- 3. Assert API Response ---
    let body: ApiResponse<Value> = response.json().await?;
    assert_eq!(
        body.result["ingested_examples"], 1,
        "Expected one example to be ingested."
    );
    assert!(body.result["message"]
        .as_str()
        .unwrap()
        .contains("completed successfully"));

    // --- 4. Assert Database State ---
    // The GitHub ingestion creates its own set of databases. We need to connect to the correct one.
    let repo_name = StorageManager::url_to_repo_name(remote_url_str);
    let db_path = format!("db/github_ingest/{repo_name}.db");

    // Make sure the directory exists before trying to connect.
    let db_dir = "db/github_ingest";
    fs::create_dir_all(db_dir)?;

    let db = Builder::new_local(&db_path).build().await?;
    let conn = db.connect()?;

    let mut stmt = conn
        .prepare("SELECT content, version, source_type FROM generated_examples")
        .await?;
    let mut rows = stmt.query(()).await?;
    let row = rows
        .next()
        .await?
        .expect("Expected to find one example in the database");

    let content: String = row.get(0)?;
    let version: String = row.get(1)?;
    let source_type: String = row.get(2)?;

    assert!(content.contains("println!(\"Hello, GitHub!\")"));
    assert_eq!(
        version, "0.1.0-test",
        "Version should have been parsed from Cargo.toml"
    );
    assert_eq!(source_type, "readme");

    Ok(())
}

#[tokio::test]
async fn test_get_examples_endpoint_success() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn().await?;
    let user_identifier = "get-examples-user@example.com";
    let token = generate_jwt(user_identifier)?;

    // Clean up from previous test runs to ensure isolation.
    let db_dir = "db/github_ingest";
    if fs::metadata(db_dir).is_ok() {
        fs::remove_dir_all(db_dir)?;
    }
    fs::create_dir_all(db_dir)?;

    // Create a mock git repository similar to the other test
    let remote_repo_dir = tempdir()?;
    let remote_repo_path = remote_repo_dir.path();
    Command::new("git")
        .arg("init")
        .arg("--bare")
        .current_dir(remote_repo_path)
        .status()?;

    let local_repo_dir = tempdir()?;
    let local_repo_path = local_repo_dir.path();
    Command::new("git")
        .arg("clone")
        .arg("--no-local")
        .arg("--depth=1")
        .arg(remote_repo_path.to_str().unwrap())
        .arg(".")
        .current_dir(local_repo_path)
        .status()?;

    fs::write(
        local_repo_path.join("README.md"),
        "```rust\nfn hello() {}\n```",
    )?;
    Command::new("git")
        .arg("-C")
        .arg(local_repo_path)
        .arg("config")
        .arg("user.email")
        .arg("test@example.com")
        .status()?;
    Command::new("git")
        .arg("-C")
        .arg(local_repo_path)
        .arg("config")
        .arg("user.name")
        .arg("Test User")
        .status()?;
    Command::new("git")
        .arg("-C")
        .arg(local_repo_path)
        .arg("add")
        .arg(".")
        .status()?;
    Command::new("git")
        .arg("-C")
        .arg(local_repo_path)
        .arg("commit")
        .arg("-m")
        .arg("commit-1")
        .status()?;
    Command::new("git")
        .arg("-C")
        .arg(local_repo_path)
        .arg("tag")
        .arg("v0.1.0")
        .status()?;
    Command::new("git")
        .arg("-C")
        .arg(local_repo_path)
        .arg("push")
        .arg("--tags")
        .arg("origin")
        .arg("master")
        .status()?;

    let remote_url_str = remote_repo_path.to_str().unwrap();

    // --- 2. Act: Ingest the repository ---
    app.client
        .post(format!("{}/ingest/github", app.address))
        .bearer_auth(token)
        .json(&json!({
            "url": remote_url_str,
            "version": "v0.1.0",
        }))
        .send()
        .await?
        .error_for_status()?;

    // --- 3. Act: Get the examples ---
    let repo_name = StorageManager::url_to_repo_name(remote_url_str);
    let response = app
        .client
        .get(format!("{}/examples/{}/v0.1.0", app.address, repo_name))
        .send()
        .await?
        .error_for_status()?;

    // --- 4. Assert ---
    let body: ApiResponse<Value> = response.json().await?;
    let content = body.result["content"].as_str().unwrap();

    assert!(content.contains("fn hello() {}"));
    assert!(content.contains("**Source:** `README.md` (`readme`)"));

    Ok(())
}

#[tokio::test]
async fn test_search_examples_e2e() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn().await?;
    let user_identifier = "search-examples-user@example.com";
    let token = generate_jwt(user_identifier)?;

    // Clean up from previous runs to ensure isolation.
    let db_dir = "db/github_ingest";
    if fs::metadata(db_dir).is_ok() {
        fs::remove_dir_all(db_dir)?;
    }
    fs::create_dir_all(db_dir)?;

    // Create a mock git repository.
    let remote_repo_dir = tempdir()?;
    let remote_repo_path = remote_repo_dir.path();
    Command::new("git")
        .args(["init", "--bare"])
        .current_dir(remote_repo_path)
        .status()?;
    let local_repo_dir = tempdir()?;
    let local_repo_path = local_repo_dir.path();
    Command::new("git")
        .args([
            "clone",
            "--no-local",
            "--depth=1",
            remote_repo_path.to_str().unwrap(),
            ".",
        ])
        .current_dir(local_repo_path)
        .status()?;
    fs::write(
        local_repo_path.join("README.md"),
        "```rust\nfn connect() {}\n```",
    )?;
    Command::new("git")
        .args([
            "-C",
            local_repo_path.to_str().unwrap(),
            "config",
            "user.email",
            "test@example.com",
        ])
        .status()?;
    Command::new("git")
        .args([
            "-C",
            local_repo_path.to_str().unwrap(),
            "config",
            "user.name",
            "Test User",
        ])
        .status()?;
    Command::new("git")
        .args(["-C", local_repo_path.to_str().unwrap(), "add", "."])
        .status()?;
    Command::new("git")
        .args([
            "-C",
            local_repo_path.to_str().unwrap(),
            "commit",
            "-m",
            "add example",
        ])
        .status()?;
    Command::new("git")
        .args(["-C", local_repo_path.to_str().unwrap(), "tag", "v1.0.0"])
        .status()?;
    Command::new("git")
        .args([
            "-C",
            local_repo_path.to_str().unwrap(),
            "push",
            "--tags",
            "origin",
            "master",
        ])
        .status()?;

    let remote_url_str = remote_repo_path.to_str().unwrap();
    let repo_name = StorageManager::url_to_repo_name(remote_url_str);

    // Ingest the repository.
    app.client
        .post(format!("{}/ingest/github", app.address))
        .bearer_auth(token.clone())
        .json(&json!({ "url": remote_url_str, "version": "v1.0.0" }))
        .send()
        .await?
        .error_for_status()?;

    // Manually add an embedding for the ingested example.
    let db_path = format!("db/github_ingest/{repo_name}.db");
    let db = Builder::new_local(&db_path).build().await?;
    let conn = db.connect()?;
    let example_id: i64 = conn
        .query("SELECT id FROM generated_examples LIMIT 1", ())
        .await?
        .next()
        .await?
        .unwrap()
        .get(0)?;
    let vector = [1.0, 0.0, 0.0]; // "connect" vector
    let vector_bytes: &[u8] =
        unsafe { std::slice::from_raw_parts(vector.as_ptr() as *const u8, vector.len() * 4) };
    conn.execute(
        "INSERT INTO example_embeddings (example_id, model_name, embedding) VALUES (?, ?, ?)",
        params![example_id, "mock-model", vector_bytes],
    )
    .await?;

    // --- 2. Act ---
    let response = app
        .client
        .post(format!("{}/search/examples", app.address))
        .bearer_auth(token)
        .json(&json!({
            "query": "how to connect?",
            "repos": [format!("{}:v1.0.0", repo_name)]
        }))
        .send()
        .await?
        .error_for_status()?;

    // --- 3. Assert ---

    let body: ApiResponse<Value> = response.json().await?;
    let results = body.result["results"].as_array().unwrap();
    assert_eq!(results.len(), 1, "Expected one search result.");
    assert!(results[0]["description"]
        .as_str()
        .unwrap()
        .contains("fn connect() {}"));

    Ok(())
}
