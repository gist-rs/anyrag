//! # Sheets Ingestor Integration Tests

mod common;

use anyhow::Result;
use anyrag::ingest::IngestionPrompts;
use anyrag::ingest::Ingestor;
use anyrag_sheets::SheetsIngestor;
use common::{MockAiProvider, TestSetup};
use httpmock::{Method, MockServer};
use serde_json::json;
use turso::{params, Value as TursoValue};

#[tokio::test]
async fn test_sheet_ingestion_yaml_workflow() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let setup = TestSetup::new().await?;
    let ai_provider = MockAiProvider::new();
    let mock_server = MockServer::start();
    let owner_id = "sheet-ingest-user-001";

    let csv_content = "question,answer\nWhat is the new feature?,It is the flux capacitor.";
    let expected_yaml = r#"
sections:
  - title: "Sheet Data"
    faqs:
      - question: "What is the new feature?"
        answer: "It is the flux capacitor."
"#;
    // Mock response for the metadata extraction LLM call.
    let mock_metadata = json!([
        {
            "type": "KEYPHRASE",
            "subtype": "CONCEPT",
            "value": "flux capacitor"
        },
        {
            "type": "CATEGORY",
            "subtype": "CONCEPT",
            "value": "Fictional Technology"
        }
    ])
    .to_string();

    // --- 2. Mock External Services ---

    // A. Mock the server that provides the CSV data.
    let sheet_serve_mock = mock_server.mock(|when, then| {
        when.method(Method::GET)
            .path("/spreadsheets/d/mock_sheet_id_12345/export")
            .query_param("format", "csv");
        then.status(200)
            .header("Content-Type", "text/csv")
            .body(csv_content);
    });

    // B. Program the Mock AI Provider with expected responses.
    // The key is a unique substring from the system prompt.
    ai_provider.add_response("expert document analyst and editor", expected_yaml);
    ai_provider.add_response("extract Category, Keyphrases, and Entities", &mock_metadata);

    // --- 3. Act: Ingest the Sheet ---
    let mock_sheet_url = format!(
        "{}/spreadsheets/d/mock_sheet_id_12345/edit",
        mock_server.base_url()
    );

    let prompts = IngestionPrompts {
        restructuring_system_prompt:
            anyrag::prompts::knowledge::KNOWLEDGE_RESTRUCTURING_SYSTEM_PROMPT,
        metadata_extraction_system_prompt:
            anyrag::prompts::tasks::KNOWLEDGE_METADATA_EXTRACTION_SYSTEM_PROMPT,
    };

    let ingestor = SheetsIngestor::new(&setup.db, &ai_provider, prompts);
    let source = json!({ "url": mock_sheet_url }).to_string();

    let result = ingestor.ingest(&source, Some(owner_id)).await?;

    // --- 4. Assert Ingestion Result & Database State ---
    assert_eq!(result.documents_added, 1);
    assert!(!result.document_ids.is_empty());
    let doc_id = &result.document_ids[0];

    let conn = setup.db.connect()?;

    // A. Assert that the document content is the structured YAML.
    let mut stmt_docs = conn
        .prepare("SELECT content FROM documents WHERE id = ?")
        .await?;
    let mut rows_docs = stmt_docs.query(params![doc_id.clone()]).await?;
    let row_doc = rows_docs.next().await?.expect("Document not found in DB");
    let stored_content: String = match row_doc.get_value(0)? {
        TursoValue::Text(s) => s,
        _ => panic!("Content was not a string"),
    };
    assert_eq!(stored_content.trim(), expected_yaml.trim());

    // B. Assert that the metadata was extracted and stored correctly.
    let mut stmt_meta = conn
        .prepare("SELECT metadata_type, metadata_value FROM content_metadata WHERE document_id = ? ORDER BY metadata_type")
        .await?;
    let mut rows_meta = stmt_meta.query(params![doc_id.clone()]).await?;

    let row1 = rows_meta.next().await?.expect("Expected metadata row 1");
    assert_eq!(row1.get::<String>(0)?, "CATEGORY");
    assert_eq!(row1.get::<String>(1)?, "Fictional Technology");

    let row2 = rows_meta.next().await?.expect("Expected metadata row 2");
    assert_eq!(row2.get::<String>(0)?, "KEYPHRASE");
    assert_eq!(row2.get::<String>(1)?, "flux capacitor");

    assert!(
        rows_meta.next().await?.is_none(),
        "Found more metadata than expected"
    );

    // --- 5. Assert Mocks Were Called ---
    sheet_serve_mock.assert();
    assert_eq!(
        ai_provider.get_calls().len(),
        2,
        "Expected 2 AI calls (restructure, metadata)"
    );

    Ok(())
}
