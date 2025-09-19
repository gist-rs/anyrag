//! # Full RAG Workflow Test for Sheet Ingestion
//!
//! This integration test simulates a complete end-to-end Retrieval-Augmented
//! Generation (RAG) workflow starting from a Google Sheet. It verifies that:
//! 1. The `/ingest/sheet` endpoint can process CSV data, restructure it via an LLM, and store it.
//! 2. The `/embed/new` endpoint can find the new documents and generate vector embeddings.
//! 3. The `/search/knowledge` endpoint can use the ingested knowledge to answer a question.

mod common;

use anyhow::Result;
use anyrag::prompts::{knowledge, tasks};
use common::{generate_jwt, TestApp};
use httpmock::Method;
use serde_json::json;

#[tokio::test]
async fn test_full_sheet_rag_workflow() -> Result<()> {
    // --- 1. Arrange ---

    let test_case_name = "test_full_sheet_rag_workflow";
    let app = TestApp::spawn(test_case_name).await?;
    let user_identifier = "rag-workflow-user@example.com";
    let token = generate_jwt(user_identifier)?;

    // --- Mock Data ---

    let csv_content = r#""Contact name","Email subject","Email body","At"
"John Smith","Inquiry for Wedding Planning Services","Hello - My partner and I are planning our wedding for June 15, 2025..."
"Emily Brown","Corporate Conference Venue and Planning","Good evening, We are looking to hold a conference for our company on November 20, 2024..."
"Michael Davis","Planning a Surprise Birthday Party","Hi there, I want to throw a surprise birthday party for my partner on December 10th, 2024...""#;

    let expected_yaml = r#"
sections:
  - title: "Contact Inquiries"
    faqs:
      - question: "Inquiry for Wedding Planning Services"
        answer: "Hello - My partner and I are planning our wedding for June 15, 2025..."
      - question: "Corporate Conference Venue and Planning"
        answer: "Good evening, We are looking to hold a conference for our company on November 20, 2024..."
      - question: "Planning a Surprise Birthday Party"
        answer: "Hi there, I want to throw a surprise birthday party for my partner on December 10th, 2024..."
"#;

    let mock_metadata = json!([
        {"type": "ENTITY", "subtype": "PERSON", "value": "John Smith"}
    ])
    .to_string();

    let mock_embeddings = json!({
        "data": [{"embedding": vec![0.1; 768], "index": 0}],
        "model": "mock-embedding-model",
    });

    let final_rag_answer =
        "Emily Brown is planning a conference for approximately 300 attendees on November 20, 2024.";

    let query_text = "Who is planning a conference and for how many people?";

    // --- 2. Mock External Services ---

    let chat_completions_path = format!("/{test_case_name}/v1/chat/completions");
    let embeddings_path = format!("/{test_case_name}/v1/embeddings");

    // --- Define Exact Payloads for Mocks ---

    let restructure_payload = json!({
        "model": "mock-gemini-model",
        "messages": [
            {"role": "system", "content": knowledge::KNOWLEDGE_RESTRUCTURING_SYSTEM_PROMPT},
            {"role": "user", "content": format!("# Markdown Content to Process:\n{}", csv_content)},
        ],
        "temperature": 0.0, "max_tokens": 8192, "stream": false
    });

    let metadata_payload = json!({
        "model": "mock-gemini-model",
        "messages": [
            {"role": "system", "content": tasks::KNOWLEDGE_METADATA_EXTRACTION_SYSTEM_PROMPT},
            {"role": "user", "content": expected_yaml.trim()},
        ],
        "temperature": 0.0, "max_tokens": 8192, "stream": false
    });

    let query_analysis_payload = json!({
        "model": "mock-gemini-model",
        "messages": [
            {"role": "system", "content": tasks::QUERY_ANALYSIS_SYSTEM_PROMPT},
            {"role": "user", "content": tasks::QUERY_ANALYSIS_USER_PROMPT.replace("{prompt}", query_text)},
        ],
        "temperature": 0.0, "max_tokens": 8192, "stream": false
    });

    // --- Configure Mocks ---

    let sheet_mock = app.mock_server.mock(|when, then| {
        when.method(Method::GET)
            .path("/spreadsheets/d/mock-rag-sheet/export");
        then.status(200).body(csv_content);
    });

    let restructure_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(chat_completions_path.clone())
            .json_body(restructure_payload);
        then.status(200).json_body(json!({
            "choices": [{"message": {"role": "assistant", "content": expected_yaml}}]
        }));
    });

    let metadata_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(chat_completions_path.clone())
            .json_body(metadata_payload);
        then.status(200).json_body(json!({
            "choices": [{"message": {"role": "assistant", "content": mock_metadata}}]
        }));
    });

    let query_analysis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(chat_completions_path.clone())
            .json_body(query_analysis_payload);
        then.status(200).json_body(json!({
            "choices": [{"message": {"role": "assistant", "content": "{\"entities\":[], \"keyphrases\":[\"conference\", \"planning\", \"people\"]}"}}]
        }));
    });

    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path(embeddings_path);
        then.status(200).json_body(mock_embeddings);
    });

    // This mock uses `body_contains` because the user prompt includes dynamic context (the retrieved docs).
    let rag_answer_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(chat_completions_path)
            .body_contains("Answer Directly First");
        then.status(200).json_body(json!({
            "choices": [{"message": {"role": "assistant", "content": final_rag_answer}}]
        }));
    });

    // --- 3. Act ---

    // Step 1: Ingest the sheet
    let mock_sheet_url = format!(
        "{}/spreadsheets/d/mock-rag-sheet/edit",
        app.mock_server.base_url()
    );
    let ingest_payload = json!({"url": mock_sheet_url});
    let ingest_response = app
        .client
        .post(app.url("/ingest/sheet"))
        .bearer_auth(token.clone())
        .json(&ingest_payload)
        .send()
        .await?;
    assert!(
        ingest_response.status().is_success(),
        "Ingestion request failed: {}",
        ingest_response.text().await?
    );

    // Step 2: Generate embeddings for the new document
    let embed_payload = json!({"limit": 10});
    let embed_response = app
        .client
        .post(app.url("/embed/new"))
        .json(&embed_payload)
        .send()
        .await?;
    assert!(
        embed_response.status().is_success(),
        "Embedding request failed: {}",
        embed_response.text().await?
    );

    // Step 3: Ask a question using the knowledge search RAG endpoint
    let search_payload = json!({
        "query": query_text,
        "use_knowledge_graph": false
    });
    let search_response = app
        .client
        .post(app.url("/search/knowledge"))
        .bearer_auth(token)
        .json(&search_payload)
        .send()
        .await?;
    assert!(
        search_response.status().is_success(),
        "Search request failed: {}",
        search_response.text().await?
    );
    let search_body: serde_json::Value = search_response.json().await?;

    // --- 4. Assert ---
    assert_eq!(search_body["result"]["text"], final_rag_answer);

    // --- 5. Verify Mocks ---
    sheet_mock.assert();
    restructure_mock.assert();
    metadata_mock.assert();
    query_analysis_mock.assert();
    embedding_mock.assert();
    rag_answer_mock.assert();

    Ok(())
}
