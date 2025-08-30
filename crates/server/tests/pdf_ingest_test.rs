//! # PDF Ingestion and RAG E2E Test
//!
//! This test verifies the entire `POST /ingest/file` workflow:
//! 1. A PDF is generated in memory with a specific, messy sentence.
//! 2. The server receives this PDF, extracts the messy text, and sends it to a mock LLM for refinement.
//! 3. The server takes the refined Markdown and sends it to a mock LLM for knowledge distillation (Q&A generation).
//! 4. The server stores the refined markdown and the Q&A pairs.
//! 5. The new Q&A pairs are embedded via a mock embedding API.
//! 6. A final RAG query (`/search/knowledge`) is made, which uses a mock LLM to synthesize an answer from the retrieved Q&A pair.
//! 7. The final answer is verified to prove the entire pipeline worked.

mod common;

use anyhow::Result;
use common::TestApp;
use httpmock::Method;
use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, Str};
use serde_json::{json, Value};

use crate::common::main;
use main::types::ApiResponse;

/// Generates a simple PDF with a specific messy sentence.
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
async fn test_pdf_ingestion_and_rag_workflow() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn().await?;

    // Define the magic sentence and the expected transformations
    let messy_sentence =
        "ThisIsA   messy    sentence. It needs refinement. The magic number is 3.14159.";
    let refined_markdown =
        "- This is a messy sentence.\n- It needs refinement.\n- The magic number is 3.14159.";
    let distilled_question = "What is the magic number?";
    let distilled_answer = "The magic number is 3.14159.";
    let final_rag_answer = "Based on the document, the magic number is 3.14159.";

    // --- 2. Generate Test PDF ---
    let pdf_data = generate_test_pdf(messy_sentence)?;

    // --- 3. Mock External Services ---
    // A. Mock the LLM Refinement call (receives raw text, returns clean markdown)
    let refinement_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("expert technical analyst"); // More robust check
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": refined_markdown}}]}),
        );
    });

    // B. Mock the Knowledge Distillation call (receives clean markdown, returns Q&A)
    let distillation_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("reconciliation agent"); // Check for the distillation prompt
        then.status(200)
            .json_body(json!({"choices": [{"message": {"role": "assistant", "content": json!({
                "faqs": [{ "question": distilled_question, "answer": distilled_answer, "is_explicit": false }],
                "content_chunks": []
            }).to_string()}}]}));
    });

    // C. Mock the Embedding API call
    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.1, 0.2, 0.3] }] }));
    });

    // D. Mock the final RAG synthesis call (receives context, returns final answer)
    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("strict, factual AI"); // Check for the RAG prompt
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_rag_answer}}]}),
        );
    });

    // --- 4. Execute Ingestion ---
    let form = reqwest::multipart::Form::new()
        .part(
            "file",
            reqwest::multipart::Part::bytes(pdf_data).file_name("test.pdf"),
        )
        .part("extractor", reqwest::multipart::Part::text("local"));

    let ingest_res = app
        .client
        .post(format!("{}/ingest/file", app.address))
        .multipart(form)
        .send()
        .await?
        .error_for_status()?;

    let ingest_body: ApiResponse<Value> = ingest_res.json().await?;
    assert_eq!(ingest_body.result["ingested_faqs"], 1);

    // --- 5. Execute Embedding ---
    app.client
        .post(format!("{}/embed/faqs/new", app.address))
        .json(&json!({ "limit": 10 }))
        .send()
        .await?
        .error_for_status()?;

    // --- 6. Execute RAG Search and Verify ---
    let search_res = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .json(&json!({ "query": "what is the magic number?" }))
        .send()
        .await?
        .error_for_status()?;

    let search_body: ApiResponse<Value> = search_res.json().await?;
    assert_eq!(search_body.result["text"], final_rag_answer);

    // --- 7. Assert Mock Calls ---
    refinement_mock.assert();
    distillation_mock.assert();
    // Embedding is called twice: once for the new FAQ, once for the search query.
    embedding_mock.assert_hits(2);
    rag_synthesis_mock.assert();

    Ok(())
}
