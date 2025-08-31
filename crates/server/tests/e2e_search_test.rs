//! # End-to-End Knowledge Search Workflow Tests
//!
//! This file tests the `/search/knowledge` endpoint, which uses the new
//! multi-stage hybrid search pipeline.

mod common;

use anyhow::Result;
use common::TestApp;
use httpmock::Method;
use serde_json::{json, Value};
use turso::{params, Builder};

use common::main::types::ApiResponse;

/// Seeds the database with two distinct documents for testing the search pipeline.
async fn seed_search_data(app: &TestApp) -> Result<()> {
    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;

    // Doc 1: PostgreSQL - Will be found via metadata
    let doc1_id = "doc_postgres";
    let doc1_content = "PostgreSQL is a powerful, open source object-relational database system.";
    let doc1_vector = [0.1, 0.1, 0.9, 0.0]; // Vector leans towards "database"
    conn.execute(
        "INSERT INTO documents (id, source_url, title, content) VALUES (?, ?, ?, ?)",
        params![
            doc1_id,
            "http://m.com/postgres",
            "PostgreSQL Info",
            doc1_content
        ],
    )
    .await?;
    conn.execute(
        "INSERT INTO content_metadata (document_id, metadata_type, metadata_subtype, metadata_value) VALUES (?, ?, ?, ?)",
        params![doc1_id, "ENTITY", "PRODUCT", "PostgreSQL"],
    ).await?;
    let doc1_vector_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(doc1_vector.as_ptr() as *const u8, doc1_vector.len() * 4)
    };
    conn.execute(
        "INSERT INTO document_embeddings (document_id, model_name, embedding) VALUES (?, ?, ?)",
        params![doc1_id, "mock-model", doc1_vector_bytes],
    )
    .await?;

    // Doc 2: Qwen3 - Should not be returned
    let doc2_id = "doc_qwen3";
    let doc2_content = "Qwen3 is a large language model.";
    let doc2_vector = [0.9, 0.1, 0.1, 0.0]; // Vector leans towards "LLM"
    conn.execute(
        "INSERT INTO documents (id, source_url, title, content) VALUES (?, ?, ?, ?)",
        params![doc2_id, "http://m.com/qwen3", "Qwen3 Info", doc2_content],
    )
    .await?;
    conn.execute(
        "INSERT INTO content_metadata (document_id, metadata_type, metadata_subtype, metadata_value) VALUES (?, ?, ?, ?)",
        params![doc2_id, "ENTITY", "PRODUCT", "Qwen3"],
    ).await?;
    let doc2_vector_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(doc2_vector.as_ptr() as *const u8, doc2_vector.len() * 4)
    };
    conn.execute(
        "INSERT INTO document_embeddings (document_id, model_name, embedding) VALUES (?, ?, ?)",
        params![doc2_id, "mock-model", doc2_vector_bytes],
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_knowledge_search_pipeline() -> Result<()> {
    // --- 1. Arrange ---
    let app = TestApp::spawn().await?;
    seed_search_data(&app).await?;

    let user_query = "Tell me about PostgreSQL";
    let final_rag_answer = "Based on the context, PostgreSQL is a powerful, open source object-relational database system.";

    // --- 2. Mock External Services ---

    // A. Mock the Query Analysis call to extract the "PostgreSQL" entity.
    let query_analysis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("expert query analyst");
        then.status(200).json_body(json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": json!({
                        "entities": ["PostgreSQL"],
                        "keyphrases": ["database"]
                    }).to_string()
                }
            }]
        }));
    });

    // B. Mock the Embedding API for the user query's vector.
    let query_vector = vec![0.1, 0.2, 0.8, 0.0]; // A vector that is semantically close to PostgreSQL doc
    let embeddings_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": query_vector }] }));
    });

    // C. Mock the final RAG synthesis.
    // Assert that it ONLY receives the PostgreSQL content.
    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("strict, factual AI")
            .body_contains("PostgreSQL is a powerful")
            .matches(|req| {
                !String::from_utf8_lossy(req.body.as_deref().unwrap_or_default()).contains("Qwen3")
            });
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_rag_answer}}]}),
        );
    });

    // --- 3. Act ---
    let response = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .json(&json!({ "query": user_query }))
        .send()
        .await?
        .error_for_status()?;

    // --- 4. Assert ---
    let response_body: ApiResponse<Value> = response.json().await?;
    assert_eq!(response_body.result["text"], final_rag_answer);

    query_analysis_mock.assert();
    embeddings_mock.assert();
    rag_synthesis_mock.assert();

    Ok(())
}
