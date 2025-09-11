//! # FAQ Flag Ingestion Tests
//!
//! This file contains integration tests for the `?faq=true` and `?faq=false`
//! query parameters across various ingestion endpoints. It verifies that the
//! server correctly chooses between "light" ingestion and the full, AI-driven
//! FAQ generation pipeline based on this flag.

mod common;

use anyhow::Result;
use anyrag_server::types::ApiResponse;
use common::{generate_jwt, TestApp};
use httpmock::Method;
use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, Str};
use serde_json::{json, Value};
use turso::{Builder, Value as TursoValue};

/// Generates a simple PDF with a specific sentence for testing.
fn generate_test_pdf(text: &str) -> Result<Vec<u8>> {
    let mut pdf = Pdf::new();
    let catalog_id = Ref::new(1);
    let page_tree_id = Ref::new(2);
    let page_id = Ref::new(3);
    let font_id = Ref::new(4);
    let content_id = Ref::new(5);
    let font_name = Name(b"F1");

    pdf.catalog(catalog_id).pages(page_tree_id);
    pdf.pages(page_tree_id).kids([page_id]).count(1);

    let mut page = pdf.page(page_id);
    page.media_box(Rect::new(0.0, 0.0, 595.0, 842.0));
    page.parent(page_tree_id);
    page.contents(content_id);
    page.resources().fonts().pair(font_name, font_id);
    page.finish();

    pdf.type1_font(font_id).base_font(Name(b"Helvetica"));

    let mut content = Content::new();
    content.begin_text();
    content.set_font(font_name, 14.0);
    content.next_line(108.0, 734.0);
    content.show(Str(text.as_bytes()));
    content.end_text();
    pdf.stream(content_id, &content.finish());

    Ok(pdf.finish())
}

#[tokio::test]
async fn test_ingest_pdf_with_faq_true() -> Result<()> {
    // Arrange
    let app = TestApp::spawn().await?;
    let token = generate_jwt("pdf-faq-user@example.com")?;
    let pdf_data = generate_test_pdf("The magic word is AnyRAG.")?;

    // Mock the full AI pipeline: refinement -> distillation -> metadata
    let refinement_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/chat/completions").body_contains("expert technical analyst");
        then.status(200).json_body(json!({"choices": [{"message": {"role": "assistant", "content": "The magic word is AnyRAG."}}]}));
    });
    let distillation_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/chat/completions").body_contains("data extraction agent");
        then.status(200).json_body(json!({"choices": [{"message": {"role": "assistant", "content": json!({
            "faqs": [{"question": "What is the magic word?", "answer": "AnyRAG", "is_explicit": false}], "content_chunks": []
        }).to_string()}}]}));
    });
    let metadata_extraction_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("expert document analyst");
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": json!({
            "metadata": [{"type": "KEYPHRASE", "subtype": "CONCEPT", "value": "AnyRAG"}]
        }).to_string()}}]}),
        );
    });
    let augmentation_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/chat/completions").body_contains("expert content analyst");
        then.status(200).json_body(json!({"choices": [{"message": {"role": "assistant", "content": json!({"augmented_faqs": []}).to_string()}}]}));
    });

    let form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(pdf_data).file_name("test.pdf"),
    );

    // Act
    let response = app
        .client
        .post(format!("{}/ingest/pdf?faq=true", app.address))
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await?
        .error_for_status()?;

    // Assert API Response
    let body: ApiResponse<Value> = response.json().await?;
    assert_eq!(body.result["ingested_faqs"], 1);
    refinement_mock.assert();
    distillation_mock.assert();
    metadata_extraction_mock.assert();
    augmentation_mock.assert_hits(0);

    // Assert Database State
    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;
    let mut stmt = conn
        .prepare(
            "SELECT question, answer FROM faq_items WHERE question = 'What is the magic word?'",
        )
        .await?;
    let mut rows = stmt.query(()).await?;
    let row = rows
        .next()
        .await?
        .expect("Expected to find the ingested PDF FAQ");
    let answer: String = row.get(1)?;
    assert_eq!(answer, "AnyRAG");

    Ok(())
}

#[tokio::test]
async fn test_ingest_sheet_with_faq_true() -> Result<()> {
    // Arrange
    let app = TestApp::spawn().await?;
    let token = generate_jwt("faq-sheet-user@example.com")?;
    let csv_data = "Question,Answer\nWhat is AnyRAG?,A RAG framework.";
    let sheet_download_mock = app.mock_server.mock(|when, then| {
        when.method(Method::GET).path_contains("/export");
        then.status(200).body(csv_data);
    });

    // Act
    let response = app
        .client
        .post(format!("{}/ingest/sheet?faq=true", app.address))
        .bearer_auth(token)
        .json(&json!({ "url": app.mock_server.url("/spreadsheets/d/mock_sheet_id/export") }))
        .send()
        .await?
        .error_for_status()?;

    // Assert API Response
    let body: ApiResponse<Value> = response.json().await?;
    assert_eq!(body.result["ingested_rows"], 1);
    assert!(body.result["message"]
        .as_str()
        .unwrap()
        .contains("Sheet FAQ ingestion successful"));
    sheet_download_mock.assert();

    // Assert Database State
    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;
    let mut stmt = conn
        .prepare("SELECT question, answer FROM faq_items WHERE question = 'What is AnyRAG?'")
        .await?;
    let mut rows = stmt.query(()).await?;
    let row = rows
        .next()
        .await?
        .expect("Expected to find the ingested FAQ");
    let answer: String = row.get(1)?;
    assert_eq!(answer, "A RAG framework.");

    Ok(())
}

#[tokio::test]
async fn test_ingest_sheet_with_faq_false() -> Result<()> {
    // Arrange
    let app = TestApp::spawn().await?;
    let token = generate_jwt("generic-sheet-user@example.com")?;
    let csv_data = "header1,header2\nvalue1,value2";
    let sheet_download_mock = app.mock_server.mock(|when, then| {
        when.method(Method::GET).path_contains("/export");
        then.status(200).body(csv_data);
    });

    // Act
    let response = app
        .client
        .post(format!("{}/ingest/sheet?faq=false", app.address))
        .bearer_auth(token)
        .json(&json!({ "url": app.mock_server.url("/spreadsheets/d/mock_sheet_id/export") }))
        .send()
        .await?
        .error_for_status()?;

    // Assert API Response
    let body: ApiResponse<Value> = response.json().await?;
    let table_name = body.result["table_name"].as_str().unwrap();
    assert_eq!(body.result["ingested_rows"], 1);
    assert!(body.result["message"]
        .as_str()
        .unwrap()
        .contains("Generic sheet ingested successfully"));
    sheet_download_mock.assert();

    // Assert Database State
    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;

    // Check that the generic table was created and populated
    let mut stmt = conn
        .prepare(&format!("SELECT header1, header2 FROM {table_name}"))
        .await?;
    let mut rows = stmt.query(()).await?;
    let row = rows
        .next()
        .await?
        .expect("Expected to find the ingested row");
    let value1: String = row.get(0)?;
    assert_eq!(value1, "value1");

    // Check that no FAQs were created
    let mut stmt_faq = conn.prepare("SELECT COUNT(*) FROM faq_items").await?;
    let mut rows_faq = stmt_faq.query(()).await?;
    let row_faq = rows_faq.next().await?.unwrap();
    let count: i64 = match row_faq.get_value(0)? {
        TursoValue::Integer(i) => i,
        _ => panic!("Expected integer"),
    };
    assert_eq!(count, 0);

    Ok(())
}

#[tokio::test]
async fn test_ingest_pdf_with_faq_false() -> Result<()> {
    // Arrange
    let app = TestApp::spawn().await?;
    let token = generate_jwt("pdf-light-user@example.com")?;
    let pdf_data = generate_test_pdf("This is a light ingestion test.")?;

    let refinement_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/chat/completions").body_contains("expert technical analyst");
        then.status(200).json_body(json!({"choices": [{"message": {"role": "assistant", "content": "This is a light ingestion test."}}]}));
    });

    let distillation_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("data extraction agent");
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": json!({
            "faqs": [], "content_chunks": []
        }).to_string()}}]}),
        );
    });

    let metadata_extraction_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("expert document analyst");
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": json!({
            "metadata": []
        }).to_string()}}]}),
        );
    });

    let augmentation_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("expert content analyst");
        then.status(200)
            .json_body(json!({"choices": [{"message": {"role": "assistant", "content": json!({"augmented_faqs": []}).to_string()}}]}));
    });

    let form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(pdf_data).file_name("light.pdf"),
    );

    // Act
    let response = app
        .client
        .post(format!("{}/ingest/pdf?faq=false", app.address))
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await?
        .error_for_status()?;

    // Assert API Response
    let body: ApiResponse<Value> = response.json().await?;
    assert_eq!(body.result["ingested_faqs"], 0);

    // Assert Mocks
    refinement_mock.assert();
    distillation_mock.assert();
    metadata_extraction_mock.assert();
    augmentation_mock.assert_hits(0);

    // Assert Database State
    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;

    // Document should be created
    let mut stmt_doc = conn
        .prepare("SELECT content FROM documents WHERE source_url = 'light.pdf'")
        .await?;
    let mut rows_doc = stmt_doc.query(()).await?;
    assert!(
        rows_doc.next().await?.is_some(),
        "Document was not created for light ingestion"
    );

    // No FAQs should be created
    let mut stmt_faq = conn.prepare("SELECT COUNT(*) FROM faq_items").await?;
    let mut rows_faq = stmt_faq.query(()).await?;
    let count: i64 = rows_faq.next().await?.unwrap().get(0)?;
    assert_eq!(count, 0);

    Ok(())
}
