//! # End-to-End Multi-Stage Hybrid Search Test
//!
//! This test verifies the complete, new hybrid search workflow as defined in `NOW.md`.
//! It ensures that the system correctly performs:
//! 1. LLM-based Query Analysis.
//! 2. Metadata Pre-filtering.
//! 3. Vector Re-ranking on the filtered candidates.
//! 4. Final RAG synthesis with the precise context.

mod common;

use anyhow::Result;
use common::TestApp;
use httpmock::Method;
use serde_json::{json, Value};
use turso::{params, Builder};

use common::main::types::ApiResponse;

/// Seeds the database with two distinct documents, their metadata, and their embeddings.
async fn seed_data_for_hybrid_search(app: &TestApp) -> Result<()> {
    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;

    // --- Document 1: Tesla Campaign ---
    let doc1_id = "doc_tesla";
    let doc1_content = "The grand prize for the campaign is a Tesla Model 3.";
    let doc1_vector = [1.0, 0.0, 0.0, 0.0];
    conn.execute(
        "INSERT INTO documents (id, source_url, title, content) VALUES (?, ?, ?, ?)",
        params![doc1_id, "http://m.com/tesla", "Tesla Prize", doc1_content],
    )
    .await?;
    conn.execute(
        "INSERT INTO content_metadata (document_id, metadata_type, metadata_subtype, metadata_value) VALUES (?, ?, ?, ?)",
        params![doc1_id, "ENTITY", "PRODUCT", "Tesla"],
    ).await?;
    conn.execute(
        "INSERT INTO content_metadata (document_id, metadata_type, metadata_subtype, metadata_value) VALUES (?, ?, ?, ?)",
        params![doc1_id, "KEYPHRASE", "CONCEPT", "campaign prize"],
    ).await?;
    let doc1_vector_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(doc1_vector.as_ptr() as *const u8, doc1_vector.len() * 4)
    };
    conn.execute(
        "INSERT INTO document_embeddings (document_id, model_name, embedding) VALUES (?, ?, ?)",
        params![doc1_id, "mock-model", doc1_vector_bytes],
    )
    .await?;

    // --- Document 2: True App Details ---
    let doc2_id = "doc_true_app";
    let doc2_content = "You must use the True App to be eligible for the campaign.";
    let doc2_vector = [0.0, 1.0, 0.0, 0.0];
    conn.execute(
        "INSERT INTO documents (id, source_url, title, content) VALUES (?, ?, ?, ?)",
        params![
            doc2_id,
            "http://m.com/true_app",
            "True App Requirement",
            doc2_content
        ],
    )
    .await?;
    conn.execute(
        "INSERT INTO content_metadata (document_id, metadata_type, metadata_subtype, metadata_value) VALUES (?, ?, ?, ?)",
        params![doc2_id, "ENTITY", "PRODUCT", "True App"],
    ).await?;
    conn.execute(
        "INSERT INTO content_metadata (document_id, metadata_type, metadata_subtype, metadata_value) VALUES (?, ?, ?, ?)",
        params![doc2_id, "KEYPHRASE", "CONCEPT", "eligibility requirement"],
    ).await?;
    let doc2_vector_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(doc2_vector.as_ptr() as *const u8, doc2_vector.len() * 4)
    };
    conn.execute(
        "INSERT INTO document_embeddings (document_id, model_name, embedding) VALUES (?, ?, ?)",
        params![doc2_id, "mock-model", doc2_vector_bytes],
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_e2e_multi_stage_hybrid_search() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let app = TestApp::spawn().await?;
    seed_data_for_hybrid_search(&app).await?;

    let user_query = "Tell me about the Tesla campaign prize";
    let final_rag_answer = "The campaign's grand prize is a Tesla Model 3.";

    // --- 2. Mock External Services ---
    // A. Mock the Query Analysis LLM call.
    // It should extract "Tesla" as an entity, which is the key to the pre-filtering stage.
    let query_analysis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("expert query analyst");
        then.status(200).json_body(json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": json!({
                        "entities": ["Tesla"],
                        "keyphrases": ["campaign prize"]
                    }).to_string()
                }
            }]
        }));
    });

    // B. Mock the Embedding API call for the user's query vector.
    // The vector is crafted to be a perfect match for the Tesla document.
    let query_vector = vec![1.0, 0.0, 0.0, 0.0];
    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": query_vector }] }));
    });

    // C. Mock the final RAG Synthesis call.
    // This is the most important assertion: we verify that the context it receives
    // *only* contains the content from the Tesla document, proving that the
    // metadata pre-filtering and vector re-ranking worked as expected.
    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("strict, factual AI") // Differentiates it from the analysis call
            .body_contains("The grand prize for the campaign is a Tesla Model 3.") // Must contain doc1 content
            .matches(|req| {
                !String::from_utf8_lossy(req.body.as_deref().unwrap_or_default())
                    .contains("You must use the True App")
            }); // MUST NOT contain doc2 content
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_rag_answer}}]}),
        );
    });

    // --- 3. Execute the search ---
    let response = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .json(&json!({ "query": user_query }))
        .send()
        .await?
        .error_for_status()?;

    // --- 4. Assert the final response and mock calls ---
    let response_body: ApiResponse<Value> = response.json().await?;
    assert_eq!(response_body.result["text"], final_rag_answer);

    query_analysis_mock.assert();
    embedding_mock.assert();
    rag_synthesis_mock.assert();

    Ok(())
}
