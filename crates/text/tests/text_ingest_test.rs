//! # Text Crate Tests
//!
//! This file contains unit and integration tests for the `anyrag-text` crate,
//! ensuring that text chunking and ingestion logic works as expected,
//! independent of the main server.

use anyhow::Result;
use anyrag::ingest::Ingestor;
use anyrag_test_utils::TestSetup;
use anyrag_text::{chunk_text, ingest_chunks_as_documents, TextIngestError, TextIngestor};
use serde_json::json;

// --- Unit Tests for chunk_text ---

#[test]
fn test_chunk_text_empty_input() {
    let result = chunk_text("   ");
    assert!(matches!(result, Err(TextIngestError::EmptyContent)));
}

#[test]
fn test_chunk_text_single_short_paragraph() -> Result<()> {
    let text = "This is a single paragraph that is well under the chunk size limit.";
    let chunks = chunk_text(text)?;
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], text);
    Ok(())
}

#[test]
fn test_chunk_text_multiple_paragraphs() -> Result<()> {
    let text = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
    let chunks = chunk_text(text)?;
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0], "First paragraph.");
    assert_eq!(chunks[1], "Second paragraph.");
    assert_eq!(chunks[2], "Third paragraph.");
    Ok(())
}

#[test]
fn test_chunk_text_long_paragraph_split() -> Result<()> {
    // CHUNK_SIZE_LIMIT is 4096, CHUNK_OVERLAP is 200.
    // 5000 chars should be split into two chunks.
    let long_text = "a".repeat(5000);
    let chunks = chunk_text(&long_text)?;

    assert_eq!(chunks.len(), 2);
    // First chunk should be exactly the limit.
    assert_eq!(chunks[0].chars().count(), 4096);
    // Second chunk should be the remainder plus the overlap.
    // The start of the second chunk is at index 4096 - 200 = 3896.
    // The length of the second chunk is 5000 - 3896 = 1104.
    assert_eq!(chunks[1].chars().count(), 1104);
    // Verify the overlap.
    assert_eq!(&chunks[0][4096 - 200..], &chunks[1][..200]);
    Ok(())
}

// --- Integration Tests for database interaction ---

#[tokio::test]
async fn test_ingest_chunks_as_documents_success() -> Result<()> {
    // --- Arrange ---
    let setup = TestSetup::new().await?;
    let mut conn = setup.db.connect()?;
    let chunks = vec![
        "This is the first chunk.".to_string(),
        "This is the second chunk.".to_string(),
    ];
    let source_id = "test_source";
    let owner_id = "test-owner-123";

    // --- Act ---
    let document_ids =
        ingest_chunks_as_documents(&mut conn, chunks, source_id, Some(owner_id)).await?;

    // --- Assert ---
    assert_eq!(document_ids.len(), 2);

    // Verify data in the database.
    let count: i64 = conn
        .query("SELECT COUNT(*) FROM documents", ())
        .await?
        .next()
        .await?
        .unwrap()
        .get(0)?;
    assert_eq!(count, 2);

    let content: String = conn
        .query(
            "SELECT content FROM documents WHERE source_url = 'test_source#chunk_1'",
            (),
        )
        .await?
        .next()
        .await?
        .unwrap()
        .get(0)?;
    assert_eq!(content, "This is the second chunk.");

    let db_owner_id: String = conn
        .query(
            "SELECT owner_id FROM documents WHERE source_url = 'test_source#chunk_0'",
            (),
        )
        .await?
        .next()
        .await?
        .unwrap()
        .get(0)?;
    assert_eq!(db_owner_id, owner_id);

    Ok(())
}

#[tokio::test]
async fn test_text_ingestor_e2e() -> Result<()> {
    // --- Arrange ---
    let setup = TestSetup::new().await?;
    let ingestor = TextIngestor::new(&setup.db);
    let owner_id = "e2e-user@test.com";

    let long_paragraph = "b".repeat(5000);
    let text_to_ingest = format!("This is the first paragraph.\n\n{long_paragraph}");
    let source = json!({
        "text": text_to_ingest,
        "source": "e2e_test"
    })
    .to_string();

    // --- Act ---
    let result = ingestor.ingest(&source, Some(owner_id)).await?;

    // --- Assert ---
    assert_eq!(result.documents_added, 3);
    assert_eq!(result.source, "e2e_test");
    assert_eq!(result.document_ids.len(), 3);

    // Verify database state
    let conn = setup.db.connect()?;
    let count: i64 = conn
        .query(
            "SELECT COUNT(*) FROM documents WHERE owner_id = ?",
            [owner_id],
        )
        .await?
        .next()
        .await?
        .unwrap()
        .get(0)?;
    assert_eq!(count, 3);

    Ok(())
}
