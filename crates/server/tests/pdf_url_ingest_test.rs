//! # PDF URL Ingestion and RAG E2E Test
//!
//! This test verifies the entire end-to-end workflow for ingesting a PDF from a URL.
//! It ensures that:
//! 1. The server can download a PDF from a given URL.
//! 2. The PDF's text is extracted and restructured into YAML via an LLM.
//! 3. Metadata is extracted from the YAML.
//! 4. The final YAML content is stored in the database.
//! 5. The new document is embedded.
//! 6. A RAG query correctly uses the structured data to synthesize an answer.

mod common;

use anyhow::Result;
use anyrag::prompts::{knowledge, tasks};
use anyrag_server::types::ApiResponse;
use anyrag_test_utils::helpers::generate_test_pdf;
use common::{generate_jwt, TestApp};
use httpmock::Method;
use serde_json::{json, Value};
use turso::{params, Builder};

#[tokio::test]
async fn test_pdf_url_ingestion_and_rag_workflow() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let test_name = "test_pdf_url_ingestion_and_rag_workflow";
    let app = TestApp::spawn(test_name).await?;
    let token = generate_jwt("pdf-url-ingest-user@example.com")?;

    let pdf_content = "The magic number from the URL is 3.14159.";
    let expected_yaml = r#"
sections:
  - title: "General Information"
    faqs:
      - question: "What is the magic number?"
        answer: "The magic number from the URL is 3.14159."
"#;
    let final_rag_answer = "Based on the document, the magic number is 3.14159.";
    let pdf_data = generate_test_pdf(pdf_content)?;

    // --- 2. Mock External Services ---
    // A. Mock the server that will host the PDF for download.
    let pdf_download_mock = app.mock_server.mock(|when, then| {
        when.method(Method::GET).path("/test.pdf");
        then.status(200)
            .header("Content-Type", "application/pdf")
            .body(&pdf_data);
    });

    let chat_completions_path = format!("/{test_name}/v1/chat/completions");

    // B. Mock the AI call for restructuring the PDF content into YAML.
    let restructuring_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(&chat_completions_path)
            .json_body_partial(
            json!({
                "messages": [
                    {"role": "system", "content": knowledge::KNOWLEDGE_RESTRUCTURING_SYSTEM_PROMPT}
                ]
            })
            .to_string(),
        );
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": expected_yaml}}]}),
        );
    });

    // C. Mock the AI call for extracting metadata from the YAML.
    let metadata_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(&chat_completions_path)
            .json_body_partial(
                json!({
                    "messages": [
                        {"role": "system", "content": tasks::KNOWLEDGE_METADATA_EXTRACTION_SYSTEM_PROMPT}
                    ]
                })
                .to_string(),
            );
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": json!([
                {"type": "KEYPHRASE", "subtype": "CONCEPT", "value": "magic number"}
            ]).to_string()}}]}),
        );
    });

    // D. Mock the Embedding API (for new doc and for search query).
    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(format!("/{test_name}/v1/embeddings"));
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.1, 0.2, 0.3] }] }));
    });

    // E. Mock the RAG Query Analysis call.
    let query_analysis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(&chat_completions_path)
            .json_body_partial(
                json!({
                    "messages": [
                        {"role": "system", "content": tasks::QUERY_ANALYSIS_SYSTEM_PROMPT}
                    ]
                })
                .to_string(),
            );
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": json!({
                "entities": [], "keyphrases": ["magic number"]
            }).to_string()}}]}),
        );
    });

    // F. Mock the final RAG Synthesis call.
    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(&chat_completions_path)
            .json_body_partial(
                json!({
                    "messages": [
                        {"role": "system", "content": tasks::RAG_SYNTHESIS_SYSTEM_PROMPT}
                    ]
                })
                .to_string(),
            );
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_rag_answer}}]}),
        );
    });

    // --- 3. Act: Ingest the PDF from the mock URL ---
    let pdf_url = app.mock_server.url("/test.pdf");
    let form = reqwest::multipart::Form::new()
        .part("url", reqwest::multipart::Part::text(pdf_url))
        .part("extractor", reqwest::multipart::Part::text("local"));

    let ingest_res = app
        .client
        .post(app.url("/ingest/pdf"))
        .bearer_auth(token.clone())
        .multipart(form)
        .send()
        .await?
        .error_for_status()?;

    let ingest_body: ApiResponse<Value> = ingest_res.json().await?;
    assert_eq!(ingest_body.result["ingested_documents"], 1);

    // --- 4. Act: Embed the new document ---
    app.client
        .post(app.url("/embed/new"))
        .json(&json!({ "limit": 10 }))
        .send()
        .await?
        .error_for_status()?;

    // --- 5. Act: Perform a RAG search ---
    let search_res = app
        .client
        .post(app.url("/search/knowledge"))
        .bearer_auth(token)
        .json(&json!({ "query": "what is the magic number?" }))
        .send()
        .await?
        .error_for_status()?;

    // --- 6. Assert API and DB State ---
    let search_body: ApiResponse<Value> = search_res.json().await?;
    assert_eq!(search_body.result["text"], final_rag_answer);

    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;
    let mut stmt = conn
        .prepare("SELECT content FROM documents WHERE source_url LIKE ?")
        .await?;
    let mut rows = stmt.query(params!["test.pdf#%"]).await?;
    let row = rows.next().await?.expect("Document chunk not found in DB");
    let stored_content: String = row.get(0)?;

    // Deserialize and check the title to ensure the correct chunk was stored.
    let parsed_chunk: Value = serde_yaml::from_str(&stored_content)?;
    assert_eq!(
        parsed_chunk["sections"][0]["title"], "General Information",
        "The title of the stored chunk is incorrect."
    );

    // --- 7. Assert All Mocks Were Called Correctly ---
    pdf_download_mock.assert();
    restructuring_mock.assert();
    metadata_mock.assert();
    embedding_mock.assert_hits(2);
    query_analysis_mock.assert();
    rag_synthesis_mock.assert();

    Ok(())
}
