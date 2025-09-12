//! # Knowledge Base Hybrid Search E2E Test
//!
//! This test verifies that the `POST /search/knowledge` endpoint correctly uses
//! a hybrid search strategy (vector + keyword) to retrieve context for the RAG pipeline.

mod common;

use anyhow::Result;
use anyrag_server::types::ApiResponse;
use common::{generate_jwt, TestApp};
use core_access::get_or_create_user;
use httpmock::Method;
use serde_json::{json, Value};
use std::path::Path;
use turso::{params, Builder};

/// A helper to manually insert and embed a FAQ into the database with a specific owner.
async fn seed_faq(
    db_path: &Path,
    owner_id: &str,
    doc_id: &str,
    question: &str,
    answer: &str,
    vector: Vec<f32>,
    metadata: Vec<(&str, &str, &str)>, // type, subtype, value
) -> Result<()> {
    let db = Builder::new_local(db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;

    conn.execute(
        "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?, ?, ?, ?, ?) ON CONFLICT(source_url) DO NOTHING",
        params![doc_id, owner_id, format!("manual_seed/{doc_id}"), question, answer],
    )
    .await?;

    conn.execute(
        "INSERT INTO faq_items (document_id, owner_id, question, answer) VALUES (?, ?, ?, ?)",
        params![doc_id, owner_id, question, answer],
    )
    .await?;

    for (m_type, m_subtype, m_value) in metadata {
        conn.execute(
            "INSERT INTO content_metadata (document_id, owner_id, metadata_type, metadata_subtype, metadata_value) VALUES (?, ?, ?, ?, ?)",
            params![doc_id, owner_id, m_type, m_subtype, m_value],
        )
        .await?;
    }

    let vector_bytes: &[u8] =
        unsafe { std::slice::from_raw_parts(vector.as_ptr() as *const u8, vector.len() * 4) };

    conn.execute(
        "INSERT INTO document_embeddings (document_id, model_name, embedding) VALUES (?, ?, ?)",
        params![doc_id, "mock-model", vector_bytes],
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_knowledge_hybrid_search_workflow() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn().await?;
    let db_path = app.db_path.clone();
    let user_identifier = "test-user-khs@example.com";

    // Create the user and get their ID to seed data correctly.
    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let user = get_or_create_user(&db, user_identifier, None).await?;

    // --- 2. Define Test Data and Vectors ---
    let faq_keyword_question = "How does the Quantum Widget work?";
    let faq_keyword_answer = "The Quantum Widget operates on principles of quantum entanglement.";
    let faq_keyword_vector = vec![1.0, 0.0, 0.0, 0.0];

    let faq_vector_question = "What is the method for advanced data processing?";
    let faq_vector_answer =
        "The method for advanced data processing involves multi-layered abstraction.";
    let faq_vector_vector = vec![0.0, 1.0, 0.0, 0.0];

    let final_rag_answer = "The Quantum Widget uses quantum entanglement, and advanced data processing uses multi-layered abstraction.";

    // --- 3. Seed the Database with data owned by our test user ---
    seed_faq(
        &db_path,
        &user.id,
        "doc_keyword",
        faq_keyword_question,
        faq_keyword_answer,
        faq_keyword_vector,
        vec![("ENTITY", "PRODUCT", "Quantum Widget")],
    )
    .await?;
    seed_faq(
        &db_path,
        &user.id,
        "doc_vector",
        faq_vector_question,
        faq_vector_answer,
        faq_vector_vector,
        vec![("KEYPHRASE", "CONCEPT", "advanced data processing")],
    )
    .await?;

    // --- 4. Mock External Services ---
    let query_analysis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
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
            .path("/v1/embeddings")
            .body_contains("complex Quantum Widget");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": vec![0.5, 0.5, 0.0, 0.0] }] }));
    });

    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("quantum entanglement")
            .body_contains("multi-layered abstraction");
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
