//! # Ownership Integration Test
//!
//! This test file verifies the core ownership logic for data ingestion and retrieval.
//! It ensures that the `owner_id` is correctly applied during data creation and
//! that the search endpoint (`/search/knowledge`) correctly filters results
//! based on the user making the request.

mod common;

use anyhow::Result;
use common::TestApp;
use core_access::get_or_create_user;
use httpmock::Method;
use serde_json::{json, Value};
use turso::{params, Builder};

use common::main::types::ApiResponse;

/// Seeds the database with documents owned by different users and one public document.
async fn seed_ownership_data(app: &TestApp) -> Result<()> {
    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;

    // 1. Create two distinct users. The handlers are hardcoded to use this user for now.
    let user_a_identifier = "default_user@example.com";
    let user_b_identifier = "another_user@example.com";
    let user_a = get_or_create_user(&db, user_a_identifier).await?;
    let user_b = get_or_create_user(&db, user_b_identifier).await?;

    // 2. Define document content
    let doc_a_id = "doc_owned_by_a";
    let doc_a_content = "This document is owned by User A.";

    let doc_b_id = "doc_owned_by_b";
    let doc_b_content = "This document is owned by User B.";

    let doc_c_id = "doc_public";
    let doc_c_content = "This document is public.";

    // 3. Insert documents with correct ownership
    conn.execute(
        "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?, ?, ?, ?, ?)",
        params![
            doc_a_id,
            user_a.id.clone(),
            "http://a.com",
            "Doc A",
            doc_a_content
        ],
    )
    .await?;
    conn.execute(
        "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?, ?, ?, ?, ?)",
        params![
            doc_b_id,
            user_b.id.clone(),
            "http://b.com",
            "Doc B",
            doc_b_content
        ],
    )
    .await?;
    conn.execute(
        "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?, ?, ?, ?, ?)",
        params![
            doc_c_id,
            None::<String>,
            "http://c.com",
            "Doc C",
            doc_c_content
        ],
    )
    .await?;

    // 4. Insert metadata for all documents so they are discoverable by the same query
    let common_keyphrase = "searchable_topic";
    let docs_with_owners = [
        (doc_a_id, Some(user_a.id.clone())),
        (doc_b_id, Some(user_b.id.clone())),
        (doc_c_id, None),
    ];

    for (doc_id, owner_id) in docs_with_owners {
        conn.execute(
            "INSERT INTO content_metadata (document_id, owner_id, metadata_type, metadata_value) VALUES (?, ?, ?, ?)",
            params![doc_id, owner_id, "KEYPHRASE", common_keyphrase],
        )
        .await?;
    }

    // 5. Insert dummy embeddings for all documents so vector search can find them.
    let vectors: [(&str, Vec<f32>); 3] = [
        (doc_a_id, vec![1.0, 0.0, 0.0]),
        (doc_b_id, vec![0.0, 1.0, 0.0]),
        (doc_c_id, vec![0.0, 0.0, 1.0]),
    ];

    for (doc_id, vector) in vectors {
        let vector_bytes: &[u8] =
            unsafe { std::slice::from_raw_parts(vector.as_ptr() as *const u8, vector.len() * 4) };
        conn.execute(
            "INSERT INTO document_embeddings (document_id, model_name, embedding) VALUES (?, ?, ?)",
            params![doc_id, "mock-model", vector_bytes],
        )
        .await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_search_respects_data_ownership() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn().await?;
    seed_ownership_data(&app).await?;

    // The user query that will be used.
    let user_query = "Find all documents about the searchable topic";
    // The final answer the mock AI will provide.
    let final_rag_answer = "Found User A's document and the public document.";

    // --- 2. Mock External Services ---
    // A. Mock the Query Analysis to find the common metadata keyphrase.
    let query_analysis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("expert query analyst");
        then.status(200).json_body(json!({
            "choices": [{"message": {"role": "assistant", "content": json!({
                "entities": [],
                "keyphrases": ["searchable_topic"]
            }).to_string()}}]
        }));
    });

    // B. Mock the Embedding API call (not critical for this test's logic, but needed for the flow).
    app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.5, 0.5, 0.5] }] }));
    });

    // C. Mock the final RAG Synthesis call.
    // THIS IS THE CORE ASSERTION OF THE TEST.
    // We verify that the context sent to the AI for the final answer contains the
    // content from the user's own document and the public document, but explicitly
    // excludes the content from the document owned by another user.
    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .body_contains("strict, factual AI")
            // It MUST contain the content of the document owned by the default user.
            .body_contains("This document is owned by User A.")
            // It MUST also contain the content of the public document.
            .body_contains("This document is public.")
            // It MUST NOT contain the content of the document owned by User B.
            .matches(|req| {
                !String::from_utf8_lossy(req.body.as_deref().unwrap_or_default())
                    .contains("This document is owned by User B.")
            });
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_rag_answer}}]}),
        );
    });

    // --- 3. Execute the search ---
    // The handler is hardcoded to act as "default_user@example.com", so we expect
    // to see that user's documents plus public ones.
    let response = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .json(&json!({ "query": user_query }))
        .send()
        .await?
        .error_for_status()?;

    // --- 4. Assert the final response and mock calls ---
    let response_body: ApiResponse<Value> = response.json().await?;
    assert_eq!(response_body.result["text"], final_rag_answer);

    query_analysis_mock.assert();
    rag_synthesis_mock.assert();

    Ok(())
}
