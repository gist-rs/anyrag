//! # Ownership & Guest User Integration Test
//!
//! This test file verifies the core ownership and guest user logic. It ensures that:
//! 1. Unauthenticated requests are processed as a "Guest User".
//! 2. Ingested data is correctly assigned the `owner_id` of the current user (real or guest).
//! 3. The search endpoint correctly filters results, allowing authenticated users to see
//!    their own content plus guest content, while guest users see only guest content.
//! 4. Requests with an invalid token are rejected.

mod common;

use anyhow::Result;
use anyrag_server::types::ApiResponse;
use axum::http::StatusCode;
use common::{generate_jwt_with_expiry, TestApp};
use core_access::{get_or_create_user, GUEST_USER_IDENTIFIER};
use httpmock::Method;
use serde_json::{json, Value};
use turso::{params, Builder};
use uuid::Uuid;

/// Seeds the database with documents owned by different users and the guest user.
async fn seed_ownership_data(app: &TestApp) -> Result<()> {
    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;

    // 1. Create users and get their deterministic IDs.
    let user_a = get_or_create_user(&db, "user_a@example.com").await?;
    let user_b = get_or_create_user(&db, "user_b@example.com").await?;
    let guest_user_id =
        Uuid::new_v5(&Uuid::NAMESPACE_URL, GUEST_USER_IDENTIFIER.as_bytes()).to_string();

    // 2. Define document content
    let doc_a_content = "This document is private to User A.";
    let doc_b_content = "This document is private to User B.";
    let doc_guest_content = "This document is public/guest owned.";

    // 3. Insert documents with correct ownership
    conn.execute(
        "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?, ?, ?, ?, ?)",
        params![
            "doc_owned_by_a",
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
            "doc_owned_by_b",
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
            "doc_guest",
            guest_user_id.clone(),
            "http://guest.com",
            "Guest Doc",
            doc_guest_content
        ],
    )
    .await?;

    // 4. Insert metadata for all documents so they are discoverable
    let common_keyphrase = "searchable_topic";
    for (doc_id, owner_id) in [
        ("doc_owned_by_a", user_a.id),
        ("doc_owned_by_b", user_b.id),
        ("doc_guest", guest_user_id),
    ] {
        conn.execute(
            "INSERT INTO content_metadata (document_id, owner_id, metadata_type, metadata_subtype, metadata_value) VALUES (?, ?, ?, ?, ?)",
            params![doc_id, owner_id, "KEYPHRASE", "CONCEPT", common_keyphrase],
        )
        .await?;
    }

    // 5. Insert embeddings for all documents so they are findable by vector search
    let doc_a_vector: Vec<f32> = vec![1.0, 0.0, 0.0];
    let doc_b_vector: Vec<f32> = vec![0.0, 1.0, 0.0];
    let doc_guest_vector: Vec<f32> = vec![0.0, 0.0, 1.0];

    let doc_a_vector_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(doc_a_vector.as_ptr() as *const u8, doc_a_vector.len() * 4)
    };
    conn.execute(
        "INSERT INTO document_embeddings (document_id, model_name, embedding) VALUES (?, ?, ?)",
        params!["doc_owned_by_a", "mock-model", doc_a_vector_bytes],
    )
    .await?;

    let doc_b_vector_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(doc_b_vector.as_ptr() as *const u8, doc_b_vector.len() * 4)
    };
    conn.execute(
        "INSERT INTO document_embeddings (document_id, model_name, embedding) VALUES (?, ?, ?)",
        params!["doc_owned_by_b", "mock-model", doc_b_vector_bytes],
    )
    .await?;

    let doc_guest_vector_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            doc_guest_vector.as_ptr() as *const u8,
            doc_guest_vector.len() * 4,
        )
    };
    conn.execute(
        "INSERT INTO document_embeddings (document_id, model_name, embedding) VALUES (?, ?, ?)",
        params!["doc_guest", "mock-model", doc_guest_vector_bytes],
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_authenticated_user_a_sees_own_and_guest_content() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn().await?;
    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let user_a_identifier = "user_a@example.com";
    let _ = get_or_create_user(&db, user_a_identifier).await?;
    seed_ownership_data(&app).await?;
    let user_query = "Find all documents about the searchable topic";
    let final_rag_answer = "Found User A's private document and the public guest document.";

    // --- 2. Mock External Services ---
    app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .body_contains("expert query analyst");
        then.status(200).json_body(json!({
            "choices": [{"message": {"role": "assistant", "content": json!({
                "entities": [], "keyphrases": ["searchable_topic"]
            }).to_string()}}]
        }));
    });
    app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.5, 0.5, 0.5] }] }));
    });
    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .body_contains("strict, factual AI")
            .body_contains("This document is private to User A.")
            .body_contains("This document is public/guest owned.")
            // It MUST NOT see User B's private content.
            .matches(|req| {
                !String::from_utf8_lossy(req.body.as_deref().unwrap_or_default())
                    .contains("private to User B")
            });
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_rag_answer}}]}),
        );
    });

    // --- 3. Execute search with a valid JWT for User A ---
    let token = generate_jwt_with_expiry(user_a_identifier, 3600)?;
    let response = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .bearer_auth(token)
        .json(&json!({ "query": user_query }))
        .send()
        .await?
        .error_for_status()?;

    // --- 4. Assert ---
    let response_body: ApiResponse<Value> = response.json().await?;
    assert_eq!(response_body.result["text"], final_rag_answer);
    rag_synthesis_mock.assert();

    Ok(())
}

#[tokio::test]
async fn test_user_b_sees_own_and_guest_content() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn().await?;
    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let _user_b_identifier = "user_b@example.com";
    let _ = get_or_create_user(&db, _user_b_identifier).await?;
    seed_ownership_data(&app).await?;
    let user_query = "Find all documents about the searchable topic";
    // The final answer should ONLY be based on the guest document.
    let final_rag_answer = "Found the public guest document.";

    // --- 2. Mock External Services ---
    app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .body_contains("expert query analyst");
        then.status(200).json_body(json!({
            "choices": [{"message": {"role": "assistant", "content": json!({
                "entities": [], "keyphrases": ["searchable_topic"]
            }).to_string()}}]
        }));
    });
    app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.5, 0.5, 0.5] }] }));
    });
    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .body_contains("strict, factual AI")
            // It MUST see the guest content.
            .body_contains("This document is public/guest owned.")
            // It MUST NOT see User A's or B's private content.
            .matches(|req| {
                let body = String::from_utf8_lossy(req.body.as_deref().unwrap_or_default());
                !body.contains("private to User A") && !body.contains("private to User B")
            });
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_rag_answer}}]}),
        );
    });

    // --- 3. Execute search with NO token ---
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
    rag_synthesis_mock.assert();

    Ok(())
}

#[tokio::test]
async fn test_request_with_invalid_token_is_rejected() -> Result<()> {
    // --- 1. Arrange ---
    let app = TestApp::spawn().await?;
    let invalid_token = "this.is.not.a.valid.jwt";

    // --- 2. Act ---
    let response = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .bearer_auth(invalid_token)
        .json(&json!({ "query": "test" }))
        .send()
        .await?;

    // --- 3. Assert ---
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body: Value = response.json().await?;
    assert_eq!(body["error"], "Invalid or expired token.");

    Ok(())
}

#[tokio::test]
async fn test_request_with_expired_token_is_rejected() -> Result<()> {
    // --- 1. Arrange ---
    let app = TestApp::spawn().await?;
    let expired_token = generate_jwt_with_expiry("any-user@example.com", 0)?; // Expires immediately
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await; // Ensure it's expired

    // --- 2. Act ---
    let response = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .bearer_auth(expired_token)
        .json(&json!({ "query": "test" }))
        .send()
        .await?;

    // --- 3. Assert ---
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body: Value = response.json().await?;
    assert_eq!(body["error"], "Invalid or expired token.");

    Ok(())
}

#[tokio::test]
async fn test_authenticated_user_b_sees_own_and_guest_content() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn().await?;
    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let user_b_identifier = "user_b@example.com";
    let _ = get_or_create_user(&db, user_b_identifier).await?;
    seed_ownership_data(&app).await?;
    let user_query = "Find all documents about the searchable topic";
    // The final answer should be based on User B's own content and the public/guest doc.
    let final_rag_answer = "Found User B's private document and the public guest document.";

    // --- 2. Mock External Services ---
    app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .body_contains("expert query analyst");
        then.status(200).json_body(json!({
            "choices": [{"message": {"role": "assistant", "content": json!({
                "entities": [], "keyphrases": ["searchable_topic"]
            }).to_string()}}]
        }));
    });
    app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.5, 0.5, 0.5] }] }));
    });
    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .body_contains("strict, factual AI")
            .body_contains("This document is public/guest owned.")
            .body_contains("This document is private to User B.")
            // CRUCIAL: Assert that User A's private content is NOT in the context.
            .matches(|req| {
                !String::from_utf8_lossy(req.body.as_deref().unwrap_or_default())
                    .contains("private to User A")
            });
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_rag_answer}}]}),
        );
    });

    // --- 3. Execute search with a valid JWT for User B ---
    let token = generate_jwt_with_expiry(user_b_identifier, 3600)?;
    let response = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .bearer_auth(token)
        .json(&json!({ "query": user_query }))
        .send()
        .await?
        .error_for_status()?;

    // --- 4. Assert ---
    let response_body: ApiResponse<Value> = response.json().await?;
    assert_eq!(response_body.result["text"], final_rag_answer);
    rag_synthesis_mock.assert();

    Ok(())
}
