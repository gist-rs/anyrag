//! # PDF Ingestor Integration Tests

use anyhow::Result;
use anyrag::{
    ingest::{IngestionPrompts, Ingestor},
    prompts::knowledge::{
        KNOWLEDGE_RESTRUCTURING_SYSTEM_PROMPT, METADATA_EXTRACTION_SYSTEM_PROMPT,
    },
};
use anyrag_pdf::PdfIngestor;
use anyrag_test_utils::{helpers::generate_test_pdf, MockAiProvider, TestSetup};
use base64::{engine::general_purpose, Engine as _};
use serde_json::json;
use turso::params;

#[tokio::test]
async fn test_pdf_ingestion_workflow() -> Result<()> {
    // --- 1. Arrange ---
    let setup = TestSetup::new().await?;
    let ai_provider = MockAiProvider::new();
    let owner_id = "pdf-ingest-user-001";
    let source_identifier = "test.pdf";

    // A. Define test content and expected LLM outputs
    let pdf_content = "The magic number is 42.";
    let expected_yaml = r#"
sections:
  - title: "First Section"
    faqs:
      - question: "What is the magic number?"
        answer: "The magic number is 42."
  - title: "Second Section"
    faqs:
      - question: "What is the answer to everything?"
        answer: "42, of course."
"#;
    let mock_metadata_1 = json!([
        { "type": "KEYPHRASE", "subtype": "CONCEPT", "value": "magic number" }
    ])
    .to_string();
    let mock_metadata_2 = json!([
        { "type": "KEYPHRASE", "subtype": "CONCEPT", "value": "everything" }
    ])
    .to_string();

    // B. Generate the PDF data
    let pdf_data = generate_test_pdf(pdf_content)?;
    let pdf_base64 = general_purpose::STANDARD.encode(&pdf_data);

    // C. Program the mock AI provider with expected responses
    // Mock responses are queued: 1 for restructure, then 1 for metadata for each chunk.
    ai_provider.add_response("expert document analyst and editor", expected_yaml);
    ai_provider.add_response("extract two types of metadata", &mock_metadata_1);
    ai_provider.add_response("extract two types of metadata", &mock_metadata_2);

    // --- 2. Act ---
    let prompts = IngestionPrompts {
        restructuring_system_prompt: KNOWLEDGE_RESTRUCTURING_SYSTEM_PROMPT,
        metadata_extraction_system_prompt: METADATA_EXTRACTION_SYSTEM_PROMPT,
    };

    let ingestor = PdfIngestor::new(&setup.db, &ai_provider, prompts);
    let source = json!({
        "source_identifier": source_identifier,
        "pdf_data_base64": pdf_base64,
        "extractor": "local"
    })
    .to_string();

    let result = ingestor.ingest(&source, Some(owner_id)).await?;

    // --- 3. Assert ---
    // A. Check the ingestion result
    assert_eq!(result.documents_added, 2, "Expected two chunks to be added");
    assert_eq!(result.source, source_identifier);

    // B. Check the database state for the two chunks
    let conn = setup.db.connect()?;
    let like_pattern = format!("{source_identifier}#%");
    let mut stmt_docs = conn
        .prepare("SELECT source_url, content, id FROM documents WHERE source_url LIKE ? ORDER BY source_url")
        .await?;
    let mut rows_docs = stmt_docs.query(params![like_pattern]).await?;

    // --- Assert Chunk 1 ---
    let row1 = rows_docs.next().await?.expect("Chunk 1 not found");
    let source_url_1 = row1.get::<String>(0)?;
    let content_1 = row1.get::<String>(1)?;
    let id_1 = row1.get::<String>(2)?;
    assert_eq!(source_url_1, "test.pdf#section_0");
    let expected_content_1 = r#"
sections:
- title: First Section
  faqs:
  - question: What is the magic number?
    answer: The magic number is 42.
"#;
    assert_eq!(content_1.trim(), expected_content_1.trim());

    // --- Assert Chunk 2 ---
    let row2 = rows_docs.next().await?.expect("Chunk 2 not found");
    let source_url_2 = row2.get::<String>(0)?;
    let content_2 = row2.get::<String>(1)?;
    let id_2 = row2.get::<String>(2)?;
    assert_eq!(source_url_2, "test.pdf#section_1");
    let expected_content_2 = r#"
sections:
- title: Second Section
  faqs:
  - question: What is the answer to everything?
    answer: 42, of course.
"#;
    assert_eq!(content_2.trim(), expected_content_2.trim());

    assert!(
        rows_docs.next().await?.is_none(),
        "Found more documents than the expected 2 chunks"
    );

    // C. Check metadata for each chunk
    // Metadata for Chunk 1
    let mut stmt_meta_1 = conn
        .prepare("SELECT metadata_value FROM content_metadata WHERE document_id = ?")
        .await?;
    let mut rows_meta_1 = stmt_meta_1.query(params![id_1]).await?;
    let meta_value_1 = rows_meta_1.next().await?.unwrap().get::<String>(0)?;
    assert_eq!(meta_value_1, "magic number");
    assert!(rows_meta_1.next().await?.is_none());

    // Metadata for Chunk 2
    let mut stmt_meta_2 = conn
        .prepare("SELECT metadata_value FROM content_metadata WHERE document_id = ?")
        .await?;
    let mut rows_meta_2 = stmt_meta_2.query(params![id_2]).await?;
    let meta_value_2 = rows_meta_2.next().await?.unwrap().get::<String>(0)?;
    assert_eq!(meta_value_2, "everything");
    assert!(rows_meta_2.next().await?.is_none());

    // D. Assert that the AI provider was called correctly
    assert_eq!(
        ai_provider.get_calls().len(),
        3,
        "Expected 3 AI calls (1 restructure, 2 metadata)"
    );

    Ok(())
}
