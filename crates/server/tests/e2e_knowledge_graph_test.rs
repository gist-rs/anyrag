//! # Knowledge Graph E2E Test
//!
//! This file contains end-to-end tests for the Knowledge Graph integration,
//! verifying both the dedicated endpoint and its integration into the main
//! hybrid search RAG pipeline.

mod common;

use anyhow::Result;
use anyrag_server::types::ApiResponse;
use chrono::{Duration, Utc};
use common::{generate_jwt, TestApp};
use core_access::get_or_create_user;
use httpmock::Method;
use serde_json::{json, Value};
use turso::{params, Builder};

#[tokio::test]
#[cfg(feature = "graph_db")]
async fn test_knowledge_graph_endpoint_e2e() -> Result<()> {
    // --- 1. Arrange ---
    let app = TestApp::spawn().await?;

    // Define time windows for the facts
    let now = Utc::now();
    let past_start = now - Duration::days(10);
    let past_end = now - Duration::days(5);
    let current_start = now - Duration::days(1);
    let current_end = now + Duration::days(1);
    let future_start = now + Duration::days(5);
    let future_end = now + Duration::days(10);

    // Seed the knowledge graph with time-sensitive data
    {
        let mut kg = app
            .knowledge_graph
            .write()
            .expect("Failed to get write lock on knowledge graph");

        kg.add_fact("Alice", "role", "Developer", past_start, past_end)?;
        kg.add_fact(
            "Alice",
            "role",
            "Lead Developer",
            current_start,
            current_end,
        )?;
        kg.add_fact("Alice", "role", "Architect", future_start, future_end)?;
    }

    // --- 2. Act ---
    // Query for the fact that should be active right now
    let response = app
        .client
        .post(format!("{}/search/knowledge_graph", app.address))
        .json(&json!({
            "subject": "Alice",
            "predicate": "role"
        }))
        .send()
        .await?;

    // --- 3. Assert ---
    assert!(response.status().is_success());

    let body: ApiResponse<Value> = response.json().await?;
    let object = body.result["object"]
        .as_str()
        .expect("Object field should be a string");

    assert_eq!(
        object, "Lead Developer",
        "The endpoint should have returned the currently active role."
    );

    Ok(())
}

#[tokio::test]
#[cfg(feature = "graph_db")]
async fn test_hybrid_search_with_knowledge_graph_context() -> Result<()> {
    // --- 1. Arrange ---
    let app = TestApp::spawn().await?;
    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;

    // Create a user to own the data
    let user_identifier = "test-user-kg@example.com";
    let user = get_or_create_user(&db, user_identifier, None).await?;

    let document_id = "doc_superwidget";
    conn.execute(
        "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?, ?, ?, ?, ?) ON CONFLICT(source_url) DO NOTHING",
        params![
            document_id,
            user.id.clone(),
            "manual_seed/superwidget",
            "SuperWidget Manual",
            "The SuperWidget is a complex device."
        ],
    )
    .await?;
    conn.execute(
        "INSERT INTO faq_items (document_id, owner_id, question, answer) VALUES (?, ?, ?, ?)",
        params![
            document_id,
            user.id.clone(),
            "What is the SuperWidget?",
            "The SuperWidget is a complex device."
        ],
    )
    .await?;

    // Seed metadata and embedding for the SuperWidget document.
    conn.execute(
        "INSERT INTO content_metadata (document_id, owner_id, metadata_type, metadata_subtype, metadata_value) VALUES (?, ?, ?, ?, ?)",
        params![document_id, user.id.clone(), "ENTITY", "PRODUCT", "SuperWidget_X500"],
    )
    .await?;

    let widget_vector: Vec<f32> = vec![0.3, 0.2, 0.1, 0.0];
    let widget_vector_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(widget_vector.as_ptr() as *const u8, widget_vector.len() * 4)
    };
    conn.execute(
        "INSERT INTO document_embeddings (document_id, model_name, embedding) VALUES (?, ?, ?)",
        params![document_id, "mock-model", widget_vector_bytes],
    )
    .await?;

    // Define a unique fact to seed the knowledge graph.
    let now = Utc::now();
    let unique_subject = "SuperWidget_X500";
    let unique_object = "The primary power source";

    {
        let mut kg = app
            .knowledge_graph
            .write()
            .expect("Failed to get write lock on KG");
        kg.add_fact(
            unique_subject,
            "role",
            unique_object,
            now - Duration::days(1),
            now + Duration::days(1),
        )?;
    }

    // Mock services
    let query_analysis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("expert query analyst");
        then.status(200).json_body(json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": json!({
                        "entities": ["SuperWidget_X500"],
                        "keyphrases": []
                    }).to_string()
                }
            }]
        }));
    });

    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/embeddings")
            .body_contains(unique_subject);
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.1, 0.2, 0.3, 0.0] }] }));
    });

    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("strict, factual AI")
            .body_matches(
                regex::Regex::new(&format!(
                    "(?s)Definitive Answer from Knowledge Graph: {}\\.",
                    regex::escape(unique_object)
                ))
                .unwrap(),
            );
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": "Acknowledged."}}]}),
        );
    });

    // --- 2. Act ---
    let token = generate_jwt(user_identifier)?;
    let response = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .bearer_auth(token)
        .json(&json!({
            "query": unique_subject,
            "use_knowledge_graph": true
        }))
        .send()
        .await?;

    // --- 3. Assert ---
    assert!(response.status().is_success(), "The API call failed.");
    query_analysis_mock.assert();
    embedding_mock.assert();
    rag_synthesis_mock.assert();

    Ok(())
}

#[tokio::test]
#[cfg(feature = "graph_db")]
async fn test_kg_provides_more_precise_answer_harry_potter() -> Result<()> {
    // --- 1. Arrange ---
    let app = TestApp::spawn().await?;
    let db = Builder::new_local(app.db_path.to_str().unwrap())
        .build()
        .await?;
    let conn = db.connect()?;
    let user_identifier = "harry-potter-fan@example.com";
    let user = get_or_create_user(&db, user_identifier, None).await?;

    // --- 2. Define Scenario Data ---
    let subject = "Harry_Potter";
    let question = "What is Harry Potter's current role?";
    let generic_answer_seed = "Harry Potter is a famous wizard known for defeating Voldemort";
    let past_role_seed = "Student at Hogwarts";
    let present_role_seed = "Head of Magical Law Enforcement";
    let future_role_seed = "Retired Auror";

    let final_answer_without_kg =
        "Based on the generic info, Harry Potter is a famous wizard known for defeating Voldemort.";
    let final_answer_with_kg = "According to the Knowledge Graph, Harry Potter's current role is Head of Magical Law Enforcement.";

    // --- 3. Seed Databases with correct ownership ---
    let document_id = "doc_wizarding_world";
    conn.execute(
        "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?, ?, ?, ?, ?) ON CONFLICT(source_url) DO NOTHING",
        params![
            document_id,
            user.id.clone(),
            "wizarding_world.txt",
            "Wizarding World Facts",
            generic_answer_seed
        ],
    )
    .await?;
    conn.execute(
        "INSERT INTO faq_items (document_id, owner_id, question, answer) VALUES (?, ?, ?, ?)",
        params![document_id, user.id.clone(), question, generic_answer_seed],
    )
    .await?;

    conn.execute(
        "INSERT INTO content_metadata (document_id, owner_id, metadata_type, metadata_subtype, metadata_value) VALUES (?, ?, ?, ?, ?)",
        params![document_id, user.id.clone(), "ENTITY", "PERSON", "Harry Potter"],
    )
    .await?;

    let generic_vector: Vec<f32> = vec![0.0, 0.0, 1.0, 0.0];
    let generic_vector_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            generic_vector.as_ptr() as *const u8,
            generic_vector.len() * 4,
        )
    };
    conn.execute(
        "INSERT INTO document_embeddings (document_id, model_name, embedding) VALUES (?, ?, ?)",
        params![document_id, "mock-model", generic_vector_bytes],
    )
    .await?;

    let now = Utc::now();
    {
        let mut kg = app.knowledge_graph.write().unwrap();
        kg.add_fact(
            subject,
            "role",
            past_role_seed,
            now - Duration::days(365),
            now - Duration::days(1),
        )?;
        kg.add_fact(
            subject,
            "role",
            present_role_seed,
            now - Duration::days(1),
            now + Duration::days(365),
        )?;
        kg.add_fact(
            subject,
            "role",
            future_role_seed,
            now + Duration::days(365),
            now + Duration::days(730),
        )?;
    }

    // --- 4. Mock External Services ---
    let query_analysis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("expert query analyst");
        then.status(200).json_body(json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": json!({
                        "entities": ["Harry Potter"],
                        "keyphrases": ["current role"]
                    }).to_string()
                }
            }]
        }));
    });

    let embedding_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST).path("/v1/embeddings");
        then.status(200)
            .json_body(json!({ "data": [{ "embedding": [0.5, 0.5, 0.5, 0.0] }] }));
    });

    let rag_without_kg_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains(generic_answer_seed)
            .matches(|req| !String::from_utf8_lossy(req.body.as_deref().unwrap_or_default()).contains("Head of Magical Law Enforcement"));
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_answer_without_kg}}]}),
        );
    });

    let rag_with_kg_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains(present_role_seed)
            .body_contains(generic_answer_seed);
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": final_answer_with_kg}}]}),
        );
    });

    // --- 5. Act & Assert ---
    let token = generate_jwt(user_identifier)?;

    // A. Call WITHOUT the Knowledge Graph
    let response_without_kg = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .bearer_auth(token.clone())
        .json(&json!({ "query": question, "use_knowledge_graph": false }))
        .send()
        .await?;
    assert!(
        response_without_kg.status().is_success(),
        "Call without KG failed"
    );
    let body_without_kg: ApiResponse<Value> = response_without_kg.json().await?;
    assert_eq!(body_without_kg.result["text"], final_answer_without_kg);
    rag_without_kg_mock.assert();

    // B. Call WITH the Knowledge Graph
    let response_with_kg = app
        .client
        .post(format!("{}/search/knowledge", app.address))
        .bearer_auth(token)
        .json(&json!({ "query": subject, "use_knowledge_graph": true }))
        .send()
        .await?;
    assert!(
        response_with_kg.status().is_success(),
        "Call with KG failed"
    );
    let body_with_kg: ApiResponse<Value> = response_with_kg.json().await?;
    assert_eq!(body_with_kg.result["text"], final_answer_with_kg);
    rag_with_kg_mock.assert();

    query_analysis_mock.assert_hits(2);
    embedding_mock.assert_hits(2);

    Ok(())
}
