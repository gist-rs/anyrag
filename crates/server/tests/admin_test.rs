//! # Admin Endpoint Tests
//!
//! This file contains integration tests for the admin-only endpoints,
//! verifying role-based access control.

mod common;

use anyhow::Result;
use anyrag_server::types::ApiResponse;
use axum::http::StatusCode;
use common::{generate_jwt, TestApp};
use core_access::get_or_create_user;
use httpmock::Method;
use serde_json::{json, Value};

#[tokio::test]
async fn test_get_users_as_root_succeeds() -> Result<()> {
    // --- 1. Arrange ---
    let app = TestApp::spawn("test_get_users_as_root_succeeds").await?;
    app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/test_get_users_as_root_succeeds/v1/chat/completions");
        then.status(200).json_body(json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Mock admin response."
                }
            }]
        }));
    });
    let root_user_identifier = "root@example.com";

    // Manually create a root user in the database, using the app's connection pool.
    let db = &app.app_state.sqlite_provider.db;
    let _ = get_or_create_user(db, root_user_identifier, Some("root")).await?;

    // Generate a token for the root user.
    let token = generate_jwt(root_user_identifier)?;

    // --- 2. Act ---
    let response = app
        .client
        .get(format!("{}/users", app.address))
        .bearer_auth(token)
        .send()
        .await?;

    // --- 3. Assert ---
    assert_eq!(response.status(), StatusCode::OK);
    let body: ApiResponse<Value> = response.json().await?;
    assert!(
        body.result.is_array(),
        "Expected the result to be an array of users"
    );
    let users = body.result.as_array().unwrap();
    assert!(!users.is_empty(), "Expected at least one user in the list");
    assert_eq!(users[0]["role"], "root");

    Ok(())
}

#[tokio::test]
async fn test_get_users_as_regular_user_fails() -> Result<()> {
    // --- 1. Arrange ---
    let app = TestApp::spawn("test_get_users_as_regular_user_fails").await?;
    app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/test_get_users_as_regular_user_fails/v1/chat/completions");
        then.status(200).json_body(json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Mock admin response."
                }
            }]
        }));
    });
    let regular_user_identifier = "user@example.com";
    let token = generate_jwt(regular_user_identifier)?;

    // --- 2. Act ---
    let response = app
        .client
        .get(format!("{}/users", app.address))
        .bearer_auth(token)
        .send()
        .await?;

    // --- 3. Assert ---
    // The current implementation returns a 500 Internal Server Error for this case.
    // A 403 Forbidden would be more appropriate, but this test verifies the current behavior.
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body: Value = response.json().await?;
    assert_eq!(body["error"], "An internal server error occurred.");

    Ok(())
}

#[tokio::test]
async fn test_get_users_as_guest_fails() -> Result<()> {
    // --- 1. Arrange ---
    let app = TestApp::spawn("test_get_users_as_guest_fails").await?;
    app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/test_get_users_as_guest_fails/v1/chat/completions");
        then.status(200).json_body(json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Mock admin response."
                }
            }]
        }));
    });

    // --- 2. Act ---
    // Make the request without an Authorization header to simulate a guest user.
    let response = app
        .client
        .get(format!("{}/users", app.address))
        .send()
        .await?;

    // --- 3. Assert ---
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body: Value = response.json().await?;
    assert_eq!(body["error"], "An internal server error occurred.");

    Ok(())
}
