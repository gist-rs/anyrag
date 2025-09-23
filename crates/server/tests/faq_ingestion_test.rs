//! # Unified Ingestion Pipeline E2E Test
//!
//! This test verifies the entire end-to-end workflow for PDF ingestion and search:
//! 1. A PDF is uploaded to `/ingest/pdf`.
//! 2. The new document is embedded via `/embed/new`.
//! 3. A RAG query is performed via `/search/knowledge`, which should retrieve
//!    the raw text as context and synthesize the final answer.

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
async fn test_unified_pdf_ingestion_and_rag_workflow() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let test_name = "test_unified_pdf_ingestion_and_rag_workflow";
    let app = TestApp::spawn(test_name).await?;
    let token = generate_jwt("unified-ingest-user@example.com")?;

    let pdf_content = "The magic word is AnyRAG. It is a powerful framework.";
    let pdf_data = generate_test_pdf(pdf_content)?;
    let final_rag_answer = "AnyRAG is a powerful framework.";

    let expected_yaml = r#"
sections:
  - title: "Test PDF Content"
    faqs:
      - question: "What is the magic word?"
        answer: "The magic word is AnyRAG. It is a powerful framework."
"#;
    let mock_metadata = json!({
        "metadata": [
            {"type": "ENTITY", "subtype": "PRODUCT", "value": "AnyRAG"}
        ]
    })
    .to_string();

    // --- 2. Mock External Services ---

    // A. Mocks for Ingestion Phase
    let restructure_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(format!("/{test_name}/v1/chat/completions"))
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

    let metadata_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(format!("/{test_name}/v1/chat/completions"))
            .json_body_partial(
                json!({
                    "messages": [
                        {"role": "system", "content": tasks::KNOWLEDGE_METADATA_EXTRACTION_SYSTEM_PROMPT}
                    ]
                })
                .to_string(),
            );
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": mock_metadata}}]}),
        );
    });

    // B. Mocks for Search/Embedding Phase
    // Mock the Embedding API (for new doc and for search query)
    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(format!("/{test_name}/v1/embeddings"));
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.1, 0.2, 0.3] }] }));
    });

    // Mock the RAG Query Analysis
    let query_analysis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(format!("/{test_name}/v1/chat/completions"))
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
                "entities": ["AnyRAG"], "keyphrases": []
            }).to_string()}}]}),
        );
    });

    // Mock the final RAG Synthesis call
    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(format!("/{test_name}/v1/chat/completions"))
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

    // --- 3. Act: Ingest the PDF ---
    let form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(pdf_data).file_name("test.pdf"),
    );

    let ingest_response = app
        .client
        .post(format!("{}/ingest/pdf", app.address))
        .bearer_auth(token.clone())
        .multipart(form)
        .send()
        .await?
        .error_for_status()?;

    // Assert API Response for ingestion
    let ingest_body: ApiResponse<Value> = ingest_response.json().await?;
    assert_eq!(ingest_body.result["ingested_documents"], 1);

    // --- 4. Assert Database State ---
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
        parsed_chunk["sections"][0]["title"], "Test PDF Content",
        "The title of the stored chunk is incorrect."
    );

    // --- 5. Act: Embed the new document ---
    app.client
        .post(format!("{}/embed/new", app.address))
        .json(&json!({ "limit": 10 }))
        .send()
        .await?
        .error_for_status()?;

    // --- 6. Act: Perform a RAG search ---
    let search_response = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .bearer_auth(token)
        .json(&json!({ "query": "What is AnyRAG?" }))
        .send()
        .await?
        .error_for_status()?;

    // Assert final RAG response
    let search_body: ApiResponse<Value> = search_response.json().await?;
    assert_eq!(search_body.result["text"], final_rag_answer);

    // --- 7. Assert All Mocks Were Called ---
    restructure_mock.assert();
    metadata_mock.assert();
    embedding_mock.assert_hits(2); // Once for ingestion, once for search
    query_analysis_mock.assert();
    rag_synthesis_mock.assert();

    Ok(())
}
