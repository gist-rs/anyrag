//! # Slot System Integration Tests
//!
//! Tests for the slot-based document routing system covering:
//! - Task 5.8: Search with active_slots returns matching + frozen docs
//! - Task 5.9: Search with empty active_slots returns only frozen docs
//! - Task 5.10: Documents not in any slot are excluded from results
//! - Task 6.6: Custom slot creation, ingestion, and routing verification
//! - Task 6.7: Reindex re-routes documents after keyword changes

mod common;

use anyhow::Result;
use anyrag::slots::SlotIngester;
use common::{generate_jwt, TestApp, TestDataBuilder};
use serde_json::{json, Value};
use std::sync::Arc;

/// Helper: create a SlotIngester connected to the test database with schema and default slots seeded.
async fn create_ingester(app: &TestApp) -> Result<SlotIngester> {
    let db = Arc::new(
        turso::Builder::new_local(app.db_path.to_str().unwrap())
            .build()
            .await?,
    );
    let ingester = SlotIngester::new(db);
    ingester.ensure_schema().await?;
    ingester.ensure_default_slots().await?;
    Ok(ingester)
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

/// Helper: extract the `result` field from an ApiResponse JSON body.
fn extract_result(body: &Value) -> &Value {
    &body["result"]
}

/// Helper: collect document IDs from a slot search or slot-documents response.
fn collect_doc_ids(results: &[Value]) -> Vec<&str> {
    results
        .iter()
        .map(|r| r["id"].as_str().unwrap_or(""))
        .collect()
}

/// Task 5.8: Search with `active_slots = ["apis"]` returns API docs + frozen architecture docs.
#[tokio::test]
async fn test_search_active_slots_returns_matching_and_frozen() -> Result<()> {
    let app = TestApp::spawn("test_5_8_search_active_slots").await?;
    let token = generate_jwt("test-user")?;
    let ingester = create_ingester(&app).await?;
    let builder = TestDataBuilder::new(&app).await?;

    // Add three documents targeting different slots
    let api_content = "pub fn handler() -> impl Responder {\n    Json(response)\n}";
    let arch_content = "// This is mod.rs — module structure overview\nmod submod;";
    let test_content = "#[test]\nfn test_something() {\n    assert_eq!(1, 1);\n}";

    builder
        .add_document("doc-api", "test-user", "API Doc", api_content, None)
        .await?
        .add_document(
            "doc-arch",
            "test-user",
            "Architecture Doc",
            arch_content,
            None,
        )
        .await?
        .add_document("doc-test", "test-user", "Test Doc", test_content, None)
        .await?;

    // Route documents through the slot ingester
    ingester.route_and_persist("doc-api", api_content).await?;
    ingester.route_and_persist("doc-arch", arch_content).await?;
    ingester.route_and_persist("doc-test", test_content).await?;

    // Search with active_slots=["apis"], include_frozen=true
    let resp = auth_post(
        &app,
        &token,
        "/search/slots",
        &json!({
            "active_slots": ["apis"],
            "include_frozen": true,
            "limit": 10
        }),
    )
    .await?;

    assert!(resp.status().is_success(), "Expected success status");
    let body: Value = resp.json().await?;
    let result = extract_result(&body);
    let results = result["results"]
        .as_array()
        .expect("results should be array");
    let result_ids = collect_doc_ids(results);

    // Assert: API doc + architecture doc (frozen) present, test doc absent
    assert!(
        result_ids.contains(&"doc-api"),
        "API doc should be in results: {result_ids:?}"
    );
    assert!(
        result_ids.contains(&"doc-arch"),
        "Architecture doc (frozen) should be in results: {result_ids:?}"
    );
    assert!(
        !result_ids.contains(&"doc-test"),
        "Test doc should NOT be in results: {result_ids:?}"
    );

    Ok(())
}

/// Task 5.9: Search with no active slots returns only frozen slot documents.
#[tokio::test]
async fn test_search_no_active_slots_returns_only_frozen() -> Result<()> {
    let app = TestApp::spawn("test_5_9_search_frozen_only").await?;
    let token = generate_jwt("test-user")?;
    let ingester = create_ingester(&app).await?;
    let builder = TestDataBuilder::new(&app).await?;

    let api_content = "pub fn handler() -> impl Responder {\n    Json(response)\n}";
    let arch_content = "// This is mod.rs — module structure overview\nmod submod;";
    let test_content = "#[test]\nfn test_something() {\n    assert_eq!(1, 1);\n}";

    builder
        .add_document("doc-api", "test-user", "API Doc", api_content, None)
        .await?
        .add_document(
            "doc-arch",
            "test-user",
            "Architecture Doc",
            arch_content,
            None,
        )
        .await?
        .add_document("doc-test", "test-user", "Test Doc", test_content, None)
        .await?;

    ingester.route_and_persist("doc-api", api_content).await?;
    ingester.route_and_persist("doc-arch", arch_content).await?;
    ingester.route_and_persist("doc-test", test_content).await?;

    // Search with empty active_slots, include_frozen=true
    let resp = auth_post(
        &app,
        &token,
        "/search/slots",
        &json!({
            "active_slots": [],
            "include_frozen": true,
            "limit": 10
        }),
    )
    .await?;

    assert!(resp.status().is_success(), "Expected success status");
    let body: Value = resp.json().await?;
    let result = extract_result(&body);
    let results = result["results"]
        .as_array()
        .expect("results should be array");
    let result_ids = collect_doc_ids(results);

    // Assert: only architecture doc (frozen) is present
    assert!(
        result_ids.contains(&"doc-arch"),
        "Architecture doc (frozen) should be in results: {result_ids:?}"
    );
    assert!(
        !result_ids.contains(&"doc-api"),
        "API doc should NOT be in results with no active slots: {result_ids:?}"
    );
    assert!(
        !result_ids.contains(&"doc-test"),
        "Test doc should NOT be in results with no active slots: {result_ids:?}"
    );

    Ok(())
}

/// Task 5.10: Slot search returns empty for documents not in any slot.
#[tokio::test]
async fn test_search_excludes_unmatched_documents() -> Result<()> {
    let app = TestApp::spawn("test_5_10_search_excludes_unmatched").await?;
    let token = generate_jwt("test-user")?;
    let ingester = create_ingester(&app).await?;
    let builder = TestDataBuilder::new(&app).await?;

    let api_content = "pub fn handler() -> impl Responder {\n    Json(response)\n}";
    let unmatched_content = "just some random text xyz with no slot keywords at all";

    builder
        .add_document("doc-api", "test-user", "API Doc", api_content, None)
        .await?
        .add_document(
            "doc-unmatched",
            "test-user",
            "Unmatched Doc",
            unmatched_content,
            None,
        )
        .await?;

    // Route both documents — the unmatched one won't match any slot
    ingester.route_and_persist("doc-api", api_content).await?;
    ingester
        .route_and_persist("doc-unmatched", unmatched_content)
        .await?;

    // Search with active_slots=["apis"], include_frozen=false to isolate the test
    let resp = auth_post(
        &app,
        &token,
        "/search/slots",
        &json!({
            "active_slots": ["apis"],
            "include_frozen": false,
            "limit": 10
        }),
    )
    .await?;

    assert!(resp.status().is_success(), "Expected success status");
    let body: Value = resp.json().await?;
    let result = extract_result(&body);
    let results = result["results"]
        .as_array()
        .expect("results should be array");
    let result_ids = collect_doc_ids(results);

    // Assert: API doc present, unmatched doc absent
    assert!(
        result_ids.contains(&"doc-api"),
        "API doc should be in results: {result_ids:?}"
    );
    assert!(
        !result_ids.contains(&"doc-unmatched"),
        "Unmatched doc should NOT be in results: {result_ids:?}"
    );

    Ok(())
}

/// Task 6.6: Create custom slot, ingest doc, verify routing via reindex.
#[tokio::test]
async fn test_custom_slot_routing() -> Result<()> {
    let app = TestApp::spawn("test_6_6_custom_slot_routing").await?;
    let token = generate_jwt("test-user")?;
    let _ingester = create_ingester(&app).await?;
    let builder = TestDataBuilder::new(&app).await?;

    // Step 1: Create a custom slot via POST /slots
    let create_resp = auth_post(
        &app,
        &token,
        "/slots",
        &json!({
            "name": "rust_errors",
            "description": "Error handling patterns in Rust",
            "decay_rate": 0.05,
            "keywords": ["Result<", "anyhow", "thiserror"]
        }),
    )
    .await?;

    assert!(
        create_resp.status().is_success(),
        "Slot creation should succeed"
    );

    // Step 2: Add document that matches custom slot keywords
    let custom_content = "use anyhow::Result;\npub fn run() -> Result<()> {\n    Ok(())\n}";
    builder
        .add_document(
            "doc-custom",
            "test-user",
            "Error Handling Doc",
            custom_content,
            None,
        )
        .await?;

    // Step 3: Reindex to route documents through the new slot
    let reindex_resp = auth_post(&app, &token, "/slots/reindex", &json!({})).await?;
    assert!(reindex_resp.status().is_success(), "Reindex should succeed");

    let reindex_body: Value = reindex_resp.json().await?;
    let reindex_result = extract_result(&reindex_body);
    let routed = reindex_result["total_documents_routed"]
        .as_u64()
        .unwrap_or(0);
    assert!(
        routed > 0,
        "At least one document should be routed, got {routed}"
    );

    // Step 4: Get slot documents for the custom slot
    let docs_resp = auth_get(&app, &token, "/slots/rust_errors/documents").await?;
    assert!(
        docs_resp.status().is_success(),
        "Get slot documents should succeed"
    );

    let docs_body: Value = docs_resp.json().await?;
    let docs = extract_result(&docs_body)
        .as_array()
        .expect("documents should be array");
    let doc_ids = collect_doc_ids(docs);

    // Assert: custom doc appears in the rust_errors slot
    assert!(
        doc_ids.contains(&"doc-custom"),
        "Document should appear in rust_errors slot: {doc_ids:?}"
    );

    Ok(())
}

/// Task 6.7: Reindex re-routes documents after keyword changes.
#[tokio::test]
async fn test_reindex_reroutes_after_keyword_changes() -> Result<()> {
    let app = TestApp::spawn("test_6_7_reindex_reroutes").await?;
    let token = generate_jwt("test-user")?;
    let ingester = create_ingester(&app).await?;
    let builder = TestDataBuilder::new(&app).await?;

    // Step 1: Add document with struct content — should match "types" slot
    let struct_content = "pub struct Foo {\n    bar: i32,\n}";
    builder
        .add_document(
            "doc-struct",
            "test-user",
            "Struct Doc",
            struct_content,
            None,
        )
        .await?;

    // Step 2: Route initially — should go to types slot
    ingester
        .route_and_persist("doc-struct", struct_content)
        .await?;

    // Verify it's in the types slot before creating the custom slot
    let types_resp = auth_get(&app, &token, "/slots/types/documents").await?;
    let types_body: Value = types_resp.json().await?;
    let types_docs = extract_result(&types_body)
        .as_array()
        .expect("should be array");
    let types_ids = collect_doc_ids(types_docs);
    assert!(
        types_ids.contains(&"doc-struct"),
        "Doc should initially be in types slot: {types_ids:?}"
    );

    // Step 3: Create a custom slot that also matches "struct "
    let create_resp = auth_post(
        &app,
        &token,
        "/slots",
        &json!({
            "name": "custom_types",
            "description": "Custom types slot with struct keyword",
            "decay_rate": 0.1,
            "keywords": ["struct "]
        }),
    )
    .await?;
    assert!(
        create_resp.status().is_success(),
        "Custom slot creation should succeed"
    );

    // Step 4: Reindex — should re-route all documents through updated slot definitions
    let reindex_resp = auth_post(&app, &token, "/slots/reindex", &json!({})).await?;
    assert!(reindex_resp.status().is_success(), "Reindex should succeed");

    // Step 5: Verify doc appears in BOTH types and custom_types slots
    let types_resp = auth_get(&app, &token, "/slots/types/documents").await?;
    let types_body: Value = types_resp.json().await?;
    let types_docs = extract_result(&types_body)
        .as_array()
        .expect("should be array");
    let types_ids = collect_doc_ids(types_docs);
    assert!(
        types_ids.contains(&"doc-struct"),
        "Doc should still be in types slot after reindex: {types_ids:?}"
    );

    let custom_resp = auth_get(&app, &token, "/slots/custom_types/documents").await?;
    let custom_body: Value = custom_resp.json().await?;
    let custom_docs = extract_result(&custom_body)
        .as_array()
        .expect("should be array");
    let custom_ids = collect_doc_ids(custom_docs);
    assert!(
        custom_ids.contains(&"doc-struct"),
        "Doc should also be in custom_types slot after reindex: {custom_ids:?}"
    );

    Ok(())
}
