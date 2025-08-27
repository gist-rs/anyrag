//! # Knowledge Base Hybrid Search E2E Test
//!
//! This test verifies that the `POST /search/knowledge` endpoint correctly uses
//! a hybrid search strategy (vector + keyword) to retrieve context for the RAG pipeline.

use anyhow::Result;
use httpmock::prelude::*;
use reqwest::Client;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};
use turso::{params, Builder};

// Include the binary's main source file to access its components.
#[path = "../src/main.rs"]
mod main;

use main::types::ApiResponse;

/// Spawns the application in the background for testing, configured with mocks.
async fn spawn_app_with_mocks(
    db_path: PathBuf,
    ai_api_url: String,
    embeddings_api_url: String,
) -> Result<String> {
    dotenvy::dotenv().ok();
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .compact()
        .try_init();

    // Set environment variables for the test instance before loading config.
    std::env::set_var("AI_API_URL", ai_api_url);
    std::env::set_var("EMBEDDINGS_API_URL", embeddings_api_url);
    std::env::set_var("EMBEDDINGS_MODEL", "mock-embedding-model");
    std::env::set_var("AI_PROVIDER", "local"); // Use the mockable provider

    let mut config = main::config::get_config().expect("Failed to load test configuration");
    config.db_url = db_path
        .to_str()
        .expect("Failed to convert temp db path to string")
        .to_string();

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{port}");

    tokio::spawn(async move {
        if let Err(e) = main::run(listener, config).await {
            eprintln!("Server error during test: {e}");
        }
    });

    sleep(Duration::from_millis(200)).await;
    Ok(address)
}

/// A helper to manually insert and embed a FAQ into the database.
async fn seed_faq(db_path: &Path, question: &str, answer: &str, vector: Vec<f32>) -> Result<()> {
    let db = Builder::new_local(db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;

    // This might fail on subsequent calls if the tables already exist, which is expected.
    let _ = anyrag::ingest::knowledge::create_kb_tables_if_not_exists(&conn).await;

    // Convert Vec<f32> to &[u8] for BLOB storage.
    let vector_bytes: &[u8] =
        unsafe { std::slice::from_raw_parts(vector.as_ptr() as *const u8, vector.len() * 4) };

    conn.execute(
        "INSERT INTO faq_kb (question, answer, source_url, is_explicit, content_hash, last_modified, embedding) VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![
            question,
            answer,
            "manual_seed",
            false,
            "hash123",
            chrono::Utc::now().to_rfc3339(),
            vector_bytes
        ],
    ).await?;

    Ok(())
}

#[tokio::test]
async fn test_knowledge_hybrid_search_workflow() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let temp_db_file = NamedTempFile::new()?;
    let db_path = temp_db_file.path().to_path_buf();
    let mock_server = MockServer::start();
    let client = Client::new();

    // --- 2. Define Test Data and Vectors ---
    // This FAQ is designed to be found by KEYWORD search for "Quantum Widget".
    let faq_keyword_question = "How does the Quantum Widget work?";
    let faq_keyword_answer = "For how to handle complex Quantum Widget information, you must know it operates on principles of quantum entanglement.";
    let faq_keyword_vector = vec![1.0, 0.0, 0.0, 0.0]; // Distinct vector

    // This FAQ is designed to be found by VECTOR search for "complex information".
    let faq_vector_question = "What is the method for advanced data processing?";
    let faq_vector_answer =
        "The method for advanced data processing involves multi-layered abstraction.";
    let faq_vector_vector = vec![0.0, 1.0, 0.0, 0.0]; // Distinct vector

    // The search query vector will be very close to the "advanced data processing" FAQ.
    let search_query_vector = vec![0.0, 0.99, 0.01, 0.0];
    let final_rag_answer = "The Quantum Widget uses quantum entanglement, and advanced data processing uses multi-layered abstraction.";

    // --- 3. Seed the Database ---
    seed_faq(
        &db_path,
        faq_keyword_question,
        faq_keyword_answer,
        faq_keyword_vector,
    )
    .await?;
    seed_faq(
        &db_path,
        faq_vector_question,
        faq_vector_answer,
        faq_vector_vector,
    )
    .await?;

    // --- 4. Mock External Services ---
    let ai_api_url = mock_server.url("/v1/chat/completions");
    let embeddings_api_url = mock_server.url("/v1/embeddings");

    // A. Mock the Embedding API call for the search query.
    let embedding_mock = mock_server.mock(|when, then| {
        when.method(POST)
            .path("/v1/embeddings")
            .body_contains("complex Quantum Widget information"); // Check for the query text
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": search_query_vector }] }));
    });

    // B. Mock the final RAG synthesis call. This is the most important assertion.
    // We will verify that the context it receives contains BOTH answers.
    let rag_synthesis_mock = mock_server.mock(|when, then| {
        when.method(POST)
            .path("/v1/chat/completions")
            // Check that the context contains key phrases from BOTH documents.
            .body_contains("multi-layered abstraction") // From the vector search result
            .body_contains("quantum entanglement"); // From the keyword search result
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_rag_answer}}]}),
        );
    });

    // --- 5. Spawn App ---
    let app_address = spawn_app_with_mocks(db_path, ai_api_url, embeddings_api_url).await?;

    // --- 6. Execute Hybrid RAG Search and Verify ---
    let search_res = client
        .post(format!("{app_address}/search/knowledge"))
        .json(&json!({ "query": "how to handle complex Quantum Widget information" }))
        .send()
        .await?
        .error_for_status()?;

    let search_body: ApiResponse<Value> = search_res.json().await?;
    assert_eq!(search_body.result["text"], final_rag_answer);

    // --- 7. Assert Mock Calls ---
    embedding_mock.assert();
    rag_synthesis_mock.assert(); // This confirms the core logic of the test.

    Ok(())
}
