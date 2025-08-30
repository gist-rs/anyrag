//! # Knowledge Base Pipeline Integration Test
//!
//! This file tests the core logic of the knowledge base system, from distillation
//! to exporting, by bypassing the initial web fetch. It mocks the LLM responses
//! and verifies data transformations and database state.

mod common;

use anyhow::Result;
use anyrag::{
    ingest::knowledge::{
        distill_and_augment, export_for_finetuning, store_structured_knowledge, RawContent,
    },
    providers::ai::local::LocalAiProvider,
};
use common::TestApp;
use httpmock::Method;
use serde_json::{json, Value};
use tracing::info;
use turso::{Builder, Value as TursoValue};

#[tokio::test]
async fn test_knowledge_ingest_and_export_pipeline() -> Result<()> {
    // --- 1. Arrange ---
    let app = TestApp::spawn().await?;
    let db_path = app.db_path.clone();

    // The test starts with pre-existing raw content.
    let page_url = "http://mock.com/page";
    let markdown_content = "# Main Title\n\n## FAQ Section\n\n**Q: What is this?**\n\nA: It is a test.\n\n## Details\n\nThis section contains important details.";
    let raw_content = RawContent {
        url: page_url.to_string(),
        markdown_content: markdown_content.to_string(),
        content_hash: format!("{:x}", md5::compute(markdown_content.as_bytes())),
    };

    // --- 2. Mock LLM Services ---
    let ai_provider =
        LocalAiProvider::new(app.mock_server.url("/v1/chat/completions"), None, None)?;

    // A. Mock the LLM Extraction call (Pass 1).
    let extraction_response = json!({
        "faqs": [{ "question": "What is this?", "answer": "It is a test.", "is_explicit": true }],
        "content_chunks": [{ "topic": "Important Details", "content": "This section contains important details." }]
    });
    let extraction_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("## FAQ Section");
        then.status(200)
            .json_body(json!({"choices": [{"message": {"role": "assistant", "content": extraction_response.to_string()}}]}));
    });

    // B. Mock the LLM Augmentation call (Pass 2).
    // This mock now returns the expected structure for `AugmentationResponse`.
    let augmentation_response = json!({
        "augmented_faqs": [{ "id": 0, "question": "What is mentioned in the details section?" }]
    });
    let augmentation_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            // Match based on content unique to the augmentation prompt.
            .body_contains("Content Chunks to Analyze");
        then.status(200)
            .json_body(json!({"choices": [{"message": {"role": "assistant", "content": augmentation_response.to_string()}}]}));
    });

    // --- 3. Act ---
    // Manually run the pipeline stages, skipping the initial fetch.
    let faq_items = distill_and_augment(&ai_provider, &raw_content).await?;
    assert_eq!(faq_items.len(), 2);
    info!("-> Distillation successful. Found 2 FAQs.");

    let db = Builder::new_local(db_path.to_str().unwrap())
        .build()
        .await?;
    anyrag::ingest::knowledge::create_kb_tables_if_not_exists(&db.connect()?).await?;

    let stored_count =
        store_structured_knowledge(&db, &raw_content.url, &raw_content.content_hash, faq_items)
            .await?;
    assert_eq!(stored_count, 2);
    info!("-> Storage successful. Stored 2 FAQs.");

    // --- 4. Assert Database State ---
    extraction_mock.assert();
    augmentation_mock.assert();

    let conn = db.connect()?;
    let mut stmt = conn
        .prepare("SELECT question, answer, is_explicit FROM faq_kb ORDER BY is_explicit DESC")
        .await?;
    let mut rows = stmt.query(()).await?;

    let row1 = rows.next().await?.unwrap();
    let (q1, a1, e1) = (
        match row1.get_value(0)? {
            TursoValue::Text(s) => s,
            v => panic!("Expected text for q1, got {v:?}"),
        },
        match row1.get_value(1)? {
            TursoValue::Text(s) => s,
            v => panic!("Expected text for a1, got {v:?}"),
        },
        match row1.get_value(2)? {
            TursoValue::Integer(i) => i,
            v => panic!("Expected integer for e1, got {v:?}"),
        },
    );
    assert_eq!(q1, "What is this?");
    assert_eq!(a1, "It is a test.");
    assert_eq!(e1, 1);

    let row2 = rows.next().await?.unwrap();
    let (q2, a2, e2) = (
        match row2.get_value(0)? {
            TursoValue::Text(s) => s,
            v => panic!("Expected text for q2, got {v:?}"),
        },
        match row2.get_value(1)? {
            TursoValue::Text(s) => s,
            v => panic!("Expected text for a2, got {v:?}"),
        },
        match row2.get_value(2)? {
            TursoValue::Integer(i) => i,
            v => panic!("Expected integer for e2, got {v:?}"),
        },
    );
    assert_eq!(q2, "What is mentioned in the details section?");
    assert_eq!(a2, "This section contains important details.");
    assert_eq!(e2, 0);
    info!("-> Database state verified successfully.");

    // --- 5. Act & Assert: Export for Fine-Tuning ---
    let export_body = export_for_finetuning(&db).await?;
    let lines: Vec<&str> = export_body.trim().lines().collect();
    assert_eq!(lines.len(), 2, "Expected two lines in the JSONL output.");

    let line1: Value = serde_json::from_str(lines[0])?;
    assert_eq!(line1["messages"][1]["content"], "What is this?");
    assert_eq!(line1["messages"][2]["content"], "It is a test.");

    info!("-> Fine-tuning export verified successfully.");

    Ok(())
}
