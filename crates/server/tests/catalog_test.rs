//! # Catalog-Driven Domain Shaping Integration Tests (Plan 007)
//!
//! Tests for the /v1/models, /v1/models/{domain}, /v1/tokenize endpoints.

mod common;

use anyhow::Result;
use common::{generate_jwt, TestApp};
use serde_json::{json, Value};

/// Helper: send an authenticated GET request.
async fn auth_get(app: &TestApp, token: &str, path: &str) -> Result<reqwest::Response> {
    let resp = app
        .client
        .get(app.url(path))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await?;
    Ok(resp)
}

/// Helper: send an authenticated POST request with a JSON body.
async fn auth_post(
    app: &TestApp,
    token: &str,
    path: &str,
    body: &Value,
) -> Result<reqwest::Response> {
    let resp = app
        .client
        .post(app.url(path))
        .header("Authorization", format!("Bearer {token}"))
        .json(body)
        .send()
        .await?;
    Ok(resp)
}

/// Test: GET /v1/models returns all configured domain experts.
#[tokio::test]
async fn test_list_domain_models() -> Result<()> {
    let app = TestApp::spawn("test_list_models").await?;
    let token = generate_jwt("test-user")?;

    let resp = auth_get(&app, &token, "/v1/models").await?;
    assert!(
        resp.status().is_success(),
        "Expected success, got {}",
        resp.status()
    );

    let result: Value = resp.json().await?;
    let models = &result["result"];

    assert!(models.is_array(), "Expected array of models");
    let models_arr = models.as_array().unwrap();

    // Should have at least 5 default domains
    assert!(
        models_arr.len() >= 5,
        "Expected >= 5 models, got {}",
        models_arr.len()
    );

    // Check that py2rs is present
    let py2rs = models_arr
        .iter()
        .find(|m| m["id"].as_str() == Some("py2rs"));
    assert!(py2rs.is_some(), "py2rs domain should be in models list");

    Ok(())
}

/// Test: GET /v1/models/py2rs returns domain metadata.
#[tokio::test]
async fn test_get_domain_model_py2rs() -> Result<()> {
    let app = TestApp::spawn("test_get_model_py2rs").await?;
    let token = generate_jwt("test-user")?;

    let resp = auth_get(&app, &token, "/v1/models/py2rs").await?;
    assert!(
        resp.status().is_success(),
        "Expected success, got {}",
        resp.status()
    );

    let result: Value = resp.json().await?;
    let model = &result["result"];

    assert_eq!(model["id"].as_str(), Some("py2rs"));
    assert!(model["keywords"].is_array());
    assert!(model["truncation"].is_object());
    assert_eq!(model["truncation"]["mode"].as_str(), Some("tokens"));
    assert_eq!(model["truncation"]["limit"].as_u64(), Some(10000));
    assert!(model["reasoning"].is_object());
    assert_eq!(
        model["reasoning"]["keep_on_tool_calls"].as_bool(),
        Some(true)
    );
    assert_eq!(model["reasoning"]["keep_on_plain"].as_bool(), Some(false));
    assert!(model["inference"].is_object());
    assert_eq!(model["inference"]["tree_budget"].as_u64(), Some(5000));
    assert_eq!(model["inference"]["draft_lookahead"].as_u64(), Some(12));
    assert!(model["hints"].is_object());
    assert_eq!(
        model["hints"]["latency_sensitivity"]
            .as_f64()
            .unwrap_or(0.0),
        0.8
    );
    assert_eq!(model["context_window"].as_u64(), Some(10000));

    Ok(())
}

/// Test: GET /v1/models/sudoku returns constrained budget.
#[tokio::test]
async fn test_get_domain_model_sudoku() -> Result<()> {
    let app = TestApp::spawn("test_get_model_sudoku").await?;
    let token = generate_jwt("test-user")?;

    let resp = auth_get(&app, &token, "/v1/models/sudoku").await?;
    assert!(resp.status().is_success());

    let result: Value = resp.json().await?;
    let model = &result["result"];

    assert_eq!(model["id"].as_str(), Some("sudoku"));
    assert_eq!(model["inference"]["tree_budget"].as_u64(), Some(100));
    assert_eq!(model["truncation"]["limit"].as_u64(), Some(4096));
    // Reasoning should be present but both false
    assert_eq!(
        model["reasoning"]["keep_on_tool_calls"].as_bool(),
        Some(false)
    );
    assert_eq!(model["reasoning"]["keep_on_plain"].as_bool(), Some(false));

    Ok(())
}

/// Test: GET /v1/models/general returns minimal metadata (no inference, no truncation).
#[tokio::test]
async fn test_get_domain_model_general() -> Result<()> {
    let app = TestApp::spawn("test_get_model_general").await?;
    let token = generate_jwt("test-user")?;

    let resp = auth_get(&app, &token, "/v1/models/general").await?;
    assert!(resp.status().is_success());

    let result: Value = resp.json().await?;
    let model = &result["result"];

    assert_eq!(model["id"].as_str(), Some("general"));
    assert!(
        model["inference"].is_null(),
        "General should have no inference budget"
    );
    assert!(
        model["truncation"].is_null(),
        "General should have no truncation policy"
    );
    assert!(
        model["reasoning"].is_null(),
        "General should have no reasoning policy"
    );
    assert!(model["hints"].is_null(), "General should have no hints");

    Ok(())
}

/// Test: GET /v1/models/nonexistent returns an error.
#[tokio::test]
async fn test_get_domain_model_not_found() -> Result<()> {
    let app = TestApp::spawn("test_model_not_found").await?;
    let token = generate_jwt("test-user")?;

    let resp = auth_get(&app, &token, "/v1/models/nonexistent_domain").await?;
    assert!(
        !resp.status().is_success(),
        "Expected error for nonexistent domain"
    );

    Ok(())
}

/// Test: POST /v1/tokenize returns token count estimate.
#[tokio::test]
async fn test_tokenize_endpoint() -> Result<()> {
    let app = TestApp::spawn("test_tokenize").await?;
    let token = generate_jwt("test-user")?;

    let body = json!({
        "text": "fn validate_token("
    });

    let resp = auth_post(&app, &token, "/v1/tokenize", &body).await?;
    assert!(
        resp.status().is_success(),
        "Expected success, got {}",
        resp.status()
    );

    let result: Value = resp.json().await?;
    assert!(
        result["token_count"].is_number(),
        "Expected token_count in response"
    );
    assert!(
        result["token_count"].as_u64().unwrap_or(0) > 0,
        "Token count should be > 0"
    );

    Ok(())
}
