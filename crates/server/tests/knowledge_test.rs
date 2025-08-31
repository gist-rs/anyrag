//! # Knowledge Base Pipeline Integration Test
//!
//! This file tests the core logic of the knowledge base system, from distillation
//! to exporting, by bypassing the initial web fetch. It mocks the LLM responses
//! and verifies data transformations and database state.

mod common;

use anyhow::Result;
use anyrag::{
    ingest::knowledge::{
        distill_and_augment, export_for_finetuning, store_structured_knowledge, IngestedDocument,
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
    let ingested_document = IngestedDocument {
        id: page_url.to_string(),
        source_url: page_url.to_string(),
        content: markdown_content.to_string(),
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
    let faq_items = distill_and_augment(&ai_provider, &ingested_document).await?;
    assert_eq!(faq_items.len(), 2);
    info!("-> Distillation successful. Found 2 FAQs.");

    let db = Builder::new_local(db_path.to_str().unwrap())
        .build()
        .await?;

    let stored_count =
        store_structured_knowledge(&db, &ingested_document.id, None, faq_items).await?;
    assert_eq!(stored_count, 2);
    info!("-> Storage successful. Stored 2 FAQs.");

    // --- 4. Assert Database State ---
    extraction_mock.assert();
    augmentation_mock.assert();

    let conn = db.connect()?;
    let mut stmt = conn
        .prepare("SELECT question, answer FROM faq_items")
        .await?;
    let mut rows = stmt.query(()).await?;

    let mut found_items = 0;
    let mut found_explicit = false;
    let mut found_augmented = false;

    while let Some(row) = rows.next().await? {
        found_items += 1;
        let q = match row.get_value(0)? {
            TursoValue::Text(s) => s,
            v => panic!("Expected text for question, got {v:?}"),
        };
        let a = match row.get_value(1)? {
            TursoValue::Text(s) => s,
            v => panic!("Expected text for answer, got {v:?}"),
        };

        if q == "What is this?" && a == "It is a test." {
            found_explicit = true;
        } else if q == "What is mentioned in the details section?"
            && a == "This section contains important details."
        {
            found_augmented = true;
        }
    }

    assert_eq!(found_items, 2, "Expected to find 2 items in the database");
    assert!(found_explicit, "Did not find the explicit FAQ");
    assert!(found_augmented, "Did not find the augmented FAQ");
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
