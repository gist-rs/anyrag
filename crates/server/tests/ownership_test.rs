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
use common::{generate_jwt_with_expiry, TestApp, TestDataBuilder};
use core_access::{get_or_create_user, GUEST_USER_IDENTIFIER};
use httpmock::Method;
use serde_json::{json, Value};

/// Seeds the database with documents owned by different users and the guest user.
async fn seed_data(app: &TestApp) -> Result<()> {
    let db = &app.app_state.sqlite_provider.db;

    // 1. Create users
    let user_a = get_or_create_user(db, "user_a@example.com", None).await?;
    let user_b = get_or_create_user(db, "user_b@example.com", None).await?;
    let guest_user = get_or_create_user(db, GUEST_USER_IDENTIFIER, None).await?;

    // 2. Seed data
    let builder = TestDataBuilder::new(app).await?;
    let common_keyphrase = "searchable_topic";
    builder
        .add_document(
            "doc_owned_by_a",
            &user_a.id,
            "Doc A",
            "This document is private to User A.",
            None,
        )
        .await?
        .add_metadata(
            "doc_owned_by_a",
            &user_a.id,
            "KEYPHRASE",
            "CONCEPT",
            common_keyphrase,
        )
        .await?
        .add_embedding("doc_owned_by_a", vec![1.0, 0.0, 0.0])
        .await?;

    builder
        .add_document(
            "doc_owned_by_b",
            &user_b.id,
            "Doc B",
            "This document is private to User B.",
            None,
        )
        .await?
        .add_metadata(
            "doc_owned_by_b",
            &user_b.id,
            "KEYPHRASE",
            "CONCEPT",
            common_keyphrase,
        )
        .await?
        .add_embedding("doc_owned_by_b", vec![0.0, 1.0, 0.0])
        .await?;

    builder
        .add_document(
            "doc_guest",
            &guest_user.id,
            "Guest Doc",
            "This document is public/guest owned.",
            None,
        )
        .await?
        .add_metadata(
            "doc_guest",
            &guest_user.id,
            "KEYPHRASE",
            "CONCEPT",
            common_keyphrase,
        )
        .await?
        .add_embedding("doc_guest", vec![0.0, 0.0, 1.0])
        .await?;

    Ok(())
}

#[tokio::test]
async fn test_authenticated_user_a_sees_own_and_guest_content() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn().await?;
    seed_data(&app).await?;
    let user_a_identifier = "user_a@example.com";
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
    seed_data(&app).await?;
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
    seed_data(&app).await?;
    let user_b_identifier = "user_b@example.com";
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
