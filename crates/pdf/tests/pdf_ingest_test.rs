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
use turso::{params, Value as TursoValue};

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
  - title: "Extracted Knowledge"
    faqs:
      - question: "What is the magic number?"
        answer: "The magic number is 42."
"#;
    let mock_metadata = json!([
        {
            "type": "KEYPHRASE",
            "subtype": "CONCEPT",
            "value": "magic number"
        }
    ])
    .to_string();

    // B. Generate the PDF data
    let pdf_data = generate_test_pdf(pdf_content)?;
    let pdf_base64 = general_purpose::STANDARD.encode(&pdf_data);

    // C. Program the mock AI provider with expected responses
    // Keyed by a unique substring from the system prompt.
    ai_provider.add_response("expert document analyst and editor", expected_yaml);
    ai_provider.add_response("extract two types of metadata", &mock_metadata);

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
    assert_eq!(result.documents_added, 1);
    assert_eq!(result.source, source_identifier);

    // B. Check the database state
    let conn = setup.db.connect()?;
    let mut stmt_docs = conn
        .prepare("SELECT content FROM documents WHERE source_url = ?")
        .await?;
    let mut rows_docs = stmt_docs.query(params![source_identifier]).await?;
    let row_doc = rows_docs.next().await?.expect("Document not found in DB");
    let stored_content: String = match row_doc.get_value(0)? {
        TursoValue::Text(s) => s,
        _ => panic!("Content was not a string"),
    };
    assert_eq!(stored_content.trim(), expected_yaml.trim());

    // C. Check the metadata state
    let doc_id_query = conn
        .query(
            "SELECT id FROM documents WHERE source_url = ?",
            params![source_identifier],
        )
        .await?
        .next()
        .await?
        .unwrap()
        .get::<String>(0)?;

    let mut stmt_meta = conn
        .prepare("SELECT metadata_type, metadata_value FROM content_metadata WHERE document_id = ?")
        .await?;
    let mut rows_meta = stmt_meta.query(params![doc_id_query]).await?;
    let row_meta = rows_meta
        .next()
        .await?
        .expect("Metadata not found for document");

    assert_eq!(row_meta.get::<String>(0)?, "KEYPHRASE");
    assert_eq!(row_meta.get::<String>(1)?, "magic number");

    assert!(
        rows_meta.next().await?.is_none(),
        "Found more metadata than expected"
    );

    // D. Assert that the AI provider was called correctly
    assert_eq!(
        ai_provider.get_calls().len(),
        2,
        "Expected 2 AI calls (restructure, metadata)"
    );

    Ok(())
}
