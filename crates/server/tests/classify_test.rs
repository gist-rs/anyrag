//! # Domain Classifier Integration Tests (Plan 005, Task 6)
//!
//! Tests for the `POST /classify/domain` endpoint covering:
//! - Keyword-based domain classification (sudoku, rust_code, py2rs)
//! - Fallback to keyword-only when AI provider unavailable
//! - Confidence scores are reasonable for obvious matches
//! - Config default domain mappings used when no candidates provided

mod common;

use anyhow::Result;
use common::{generate_jwt, TestApp};
use serde_json::{json, Value};

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

/// Helper: extract the `result` field from an ApiResponse JSON body.
fn extract_result(body: &Value) -> &Value {
    &body["result"]
}

/// Test: classify "solve this sudoku" → domain "sudoku".
///
/// The prompt contains the keyword "sudoku" which should give
/// a high keyword score for the sudoku domain.
#[tokio::test]
async fn test_classify_sudoku_prompt() -> Result<()> {
    let app = TestApp::spawn("test_classify_sudoku").await?;
    let token = generate_jwt("test-user")?;

    let body = json!({
        "prompt": "solve this sudoku puzzle for me",
        "candidate_domains": [
            {
                "name": "sudoku",
                "keywords": ["sudoku", "puzzle", "grid", "9x9", "digit"],
                "slots": []
            },
            {
                "name": "rust_code",
                "keywords": ["rust", "cargo", "axum", "tokio", "trait", "impl", "compile"],
                "slots": []
            },
            {
                "name": "py2rs",
                "keywords": ["python", "rewrite", "fastapi", "flask", "translate"],
                "slots": []
            },
            {
                "name": "general",
                "keywords": [],
                "slots": []
            }
        ]
    });

    let resp = auth_post(&app, &token, "/classify/domain", &body).await?;
    assert!(
        resp.status().is_success(),
        "Expected success, got {}",
        resp.status()
    );

    let result: Value = resp.json().await?;
    let classification = extract_result(&result);

    assert_eq!(classification["domain"].as_str(), Some("sudoku"));
    assert!(
        classification["confidence"].as_f64().unwrap_or(0.0) > 0.1,
        "Confidence should be > 0.1 for obvious sudoku match, got {:?}",
        classification["confidence"]
    );

    Ok(())
}

/// Test: classify "write Rust HTTP server" → domain "rust_code".
///
/// The prompt contains keywords "rust" which maps to rust_code domain.
#[tokio::test]
async fn test_classify_rust_code_prompt() -> Result<()> {
    let app = TestApp::spawn("test_classify_rust_code").await?;
    let token = generate_jwt("test-user")?;

    let body = json!({
        "prompt": "write a Rust HTTP server with axum and tokio",
        "candidate_domains": [
            {
                "name": "sudoku",
                "keywords": ["sudoku", "puzzle", "grid", "9x9", "digit"],
                "slots": []
            },
            {
                "name": "rust_code",
                "keywords": ["rust", "cargo", "axum", "tokio", "trait", "impl", "compile"],
                "slots": []
            },
            {
                "name": "py2rs",
                "keywords": ["python", "rewrite", "fastapi", "flask", "translate"],
                "slots": []
            },
            {
                "name": "general",
                "keywords": [],
                "slots": []
            }
        ]
    });

    let resp = auth_post(&app, &token, "/classify/domain", &body).await?;
    assert!(
        resp.status().is_success(),
        "Expected success, got {}",
        resp.status()
    );

    let result: Value = resp.json().await?;
    let classification = extract_result(&result);

    assert_eq!(classification["domain"].as_str(), Some("rust_code"));
    assert!(
        classification["confidence"].as_f64().unwrap_or(0.0) > 0.1,
        "Confidence should be > 0.1 for obvious rust match, got {:?}",
        classification["confidence"]
    );

    Ok(())
}

/// Test: classify "translate FastAPI to Axum" → domain "py2rs".
///
/// The prompt contains keywords from both py2rs ("fastapi", "translate")
/// and rust_code ("axum"), but py2rs should win with higher overlap.
#[tokio::test]
async fn test_classify_py2rs_prompt() -> Result<()> {
    let app = TestApp::spawn("test_classify_py2rs").await?;
    let token = generate_jwt("test-user")?;

    let body = json!({
        "prompt": "translate this FastAPI endpoint to Rust",
        "candidate_domains": [
            {
                "name": "sudoku",
                "keywords": ["sudoku", "puzzle", "grid", "9x9", "digit"],
                "slots": []
            },
            {
                "name": "rust_code",
                "keywords": ["rust", "cargo", "axum", "tokio", "trait", "impl", "compile"],
                "slots": []
            },
            {
                "name": "py2rs",
                "keywords": ["python", "rewrite", "fastapi", "flask", "translate"],
                "slots": []
            },
            {
                "name": "general",
                "keywords": [],
                "slots": []
            }
        ]
    });

    let resp = auth_post(&app, &token, "/classify/domain", &body).await?;
    assert!(
        resp.status().is_success(),
        "Expected success, got {}",
        resp.status()
    );

    let result: Value = resp.json().await?;
    let classification = extract_result(&result);

    // py2rs has "fastapi" + "translate" = 2/5 keywords matched
    // rust_code has "rust" = 1/7 keywords matched
    // py2rs should win on keyword score alone
    assert_eq!(
        classification["domain"].as_str(),
        Some("py2rs"),
        "Expected py2rs to win with higher keyword overlap"
    );
    assert!(
        classification["confidence"].as_f64().unwrap_or(0.0) > 0.1,
        "Confidence should be > 0.1 for py2rs match, got {:?}",
        classification["confidence"]
    );

    Ok(())
}

/// Test: fallback to keyword-only when AI provider unavailable.
///
/// The test server uses a mock server for embedding, but no mock is set up
/// for the embeddings endpoint. This means embedding scoring should fail
/// gracefully and fall back to keyword-only scoring.
#[tokio::test]
async fn test_keyword_fallback_when_provider_unavailable() -> Result<()> {
    let app = TestApp::spawn("test_classify_keyword_fallback").await?;
    let token = generate_jwt("test-user")?;

    // Domains with slots configured — the handler will attempt embedding scoring
    // but the mock server won't have a matching mock, so it falls back to keyword-only
    let body = json!({
        "prompt": "solve this sudoku puzzle",
        "candidate_domains": [
            {
                "name": "sudoku",
                "keywords": ["sudoku", "puzzle", "grid", "9x9"],
                "slots": ["tests"]
            },
            {
                "name": "rust_code",
                "keywords": ["rust", "cargo", "axum", "tokio"],
                "slots": ["apis", "types"]
            }
        ]
    });

    let resp = auth_post(&app, &token, "/classify/domain", &body).await?;
    assert!(
        resp.status().is_success(),
        "Expected success even when embedding fails, got {}",
        resp.status()
    );

    let result: Value = resp.json().await?;
    let classification = extract_result(&result);

    // Should still classify correctly using keyword-only fallback
    assert_eq!(classification["domain"].as_str(), Some("sudoku"));
    assert!(
        classification["confidence"].as_f64().unwrap_or(0.0) > 0.1,
        "Keyword-only confidence should be > 0.1 for obvious match, got {:?}",
        classification["confidence"]
    );

    Ok(())
}

/// Test: confidence scores are reasonable for obvious matches.
///
/// An obvious match (all keywords present) should have high confidence (>0.5),
/// while a domain with no keyword overlap should have 0.0 confidence.
#[tokio::test]
async fn test_confidence_scores_reasonable() -> Result<()> {
    let app = TestApp::spawn("test_classify_confidence").await?;
    let token = generate_jwt("test-user")?;

    let body = json!({
        "prompt": "sudoku puzzle grid digit",
        "candidate_domains": [
            {
                "name": "sudoku",
                "keywords": ["sudoku", "puzzle", "grid", "9x9", "digit"],
                "slots": []
            },
            {
                "name": "rust_code",
                "keywords": ["rust", "cargo", "axum", "tokio"],
                "slots": []
            }
        ]
    });

    let resp = auth_post(&app, &token, "/classify/domain", &body).await?;
    assert!(resp.status().is_success());

    let result: Value = resp.json().await?;
    let classification = extract_result(&result);

    // All 4 of 5 sudoku keywords present → high confidence (keyword-only = 0.8)
    let confidence = classification["confidence"].as_f64().unwrap_or(0.0);
    assert!(
        confidence >= 0.5,
        "Obvious match should have confidence >= 0.5, got {confidence}"
    );

    // Alternatives should have lower confidence
    let alternatives = classification["alternatives"]
        .as_array()
        .expect("alternatives array");
    let rust_alt = alternatives
        .iter()
        .find(|a| a["domain"].as_str() == Some("rust_code"))
        .expect("rust_code alternative");

    let rust_confidence = rust_alt["confidence"].as_f64().unwrap_or(1.0);
    assert!(
        rust_confidence < confidence,
        "Alternative confidence ({rust_confidence}) should be less than top ({confidence})"
    );

    Ok(())
}

/// Test: uses config default domain mappings when no candidates provided.
///
/// When `candidate_domains` is empty/missing, the handler should fall back
/// to the server's configured `domain_mappings` from `AppConfig`.
#[tokio::test]
async fn test_uses_config_defaults_when_no_candidates() -> Result<()> {
    let app = TestApp::spawn("test_classify_config_defaults").await?;
    let token = generate_jwt("test-user")?;

    // Send request without candidate_domains
    let body = json!({
        "prompt": "solve this sudoku puzzle"
    });

    let resp = auth_post(&app, &token, "/classify/domain", &body).await?;
    assert!(
        resp.status().is_success(),
        "Expected success with config defaults, got {}",
        resp.status()
    );

    let result: Value = resp.json().await?;
    let classification = extract_result(&result);

    // Should use default config mappings (which include sudoku)
    assert_eq!(
        classification["domain"].as_str(),
        Some("sudoku"),
        "Should classify as sudoku using config defaults"
    );
    assert!(
        classification["confidence"].as_f64().unwrap_or(0.0) > 0.1,
        "Config default classification should have reasonable confidence"
    );

    Ok(())
}

/// Test: empty prompt with domains returns a valid classification.
///
/// Edge case: an empty prompt should still produce a result
/// (all keyword scores will be 0.0, first domain wins by default).
#[tokio::test]
async fn test_empty_prompt_still_classifies() -> Result<()> {
    let app = TestApp::spawn("test_classify_empty_prompt").await?;
    let token = generate_jwt("test-user")?;

    let body = json!({
        "prompt": "",
        "candidate_domains": [
            {
                "name": "sudoku",
                "keywords": ["sudoku"],
                "slots": []
            },
            {
                "name": "rust_code",
                "keywords": ["rust"],
                "slots": []
            }
        ]
    });

    let resp = auth_post(&app, &token, "/classify/domain", &body).await?;
    assert!(
        resp.status().is_success(),
        "Expected success even with empty prompt, got {}",
        resp.status()
    );

    let result: Value = resp.json().await?;
    let classification = extract_result(&result);

    // Should return some domain even with no keyword matches
    assert!(
        classification["domain"].as_str().is_some(),
        "Should return a domain even with empty prompt"
    );

    Ok(())
}
