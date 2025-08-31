//! Example: End-to-end Knowledge Base RAG workflow.
//!
//! This example demonstrates the full "virtuous cycle" workflow:
//! 1.  It ingests content from a real-world URL into the knowledge base.
//! 2.  It generates vector embeddings for the newly created FAQs.
//! 3.  It uses the RAG pattern (`/search/knowledge`) to ask questions against
//!     that knowledge and get synthesized, natural-language answers.
//!
//! # Prerequisites
//!
//! - A valid `.env` file in the `crates/server` directory with credentials
//!   for a running AI provider (e.g., a local Ollama server).
//! - An internet connection to fetch the URL.
//!
//! # Usage
//!
//! From the workspace root (`anyrag/`):
//! `cargo run -p anyrag-server --example knowledge_prompt`

// Include the binary's main source file to access its components.
#[path = "../src/main.rs"]
mod main;

use anyhow::Result;
use axum::{extract::Query, Json};
use main::{
    handlers::{self, EmbedNewRequest, IngestRequest, SearchRequest},
    state::AppState,
};

use std::{fs, time::Duration};
use tokio::time::sleep;
use tracing::info;
use tracing_subscriber::EnvFilter;

/// Cleans up database files for a fresh run.
async fn cleanup_db(db_path: &str) -> Result<()> {
    for path in [db_path, &format!("{db_path}-wal")] {
        if fs::metadata(path).is_ok() {
            fs::remove_file(path)?;
            info!("Removed existing database file: {}", path);
        }
    }
    Ok(())
}

/// A helper function to call the knowledge search RAG endpoint.
async fn ask_question(
    app_state: AppState,
    query: &str,
    instruction: Option<&str>,
) -> Result<String> {
    info!("--- Asking Question: '{}' ---", query);

    let payload = SearchRequest {
        query: query.to_string(),
        instruction: instruction.map(String::from),
        limit: Some(5), // How many KB entries to use for context

        use_knowledge_graph: Some(true),
    };

    let result = handlers::knowledge_search_handler(
        axum::extract::State(app_state),
        Query(main::types::DebugParams::default()),
        Json(payload),
    )
    .await;

    match result {
        Ok(Json(response)) => Ok(response.result.text),
        Err(e) => anyhow::bail!("Error occurred while asking question: {:?}", e),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // --- 1. Setup ---
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    dotenvy::dotenv().ok();
    info!("Environment variables loaded.");

    let db_path = "db/anyrag.db";
    cleanup_db(db_path).await?;

    let config =
        main::config::get_config().expect("Failed to load configuration. Is .env present?");
    let app_state = main::state::build_app_state(config).await?;
    info!("Application state built successfully.");

    sleep(Duration::from_millis(100)).await;

    // --- 2. Ingest Knowledge ---
    info!("--- Starting Knowledge Ingestion ---");
    let ingest_url = "https://www.true.th/betterliv/support/true-app-mega-campaign";
    let ingest_payload = IngestRequest {
        url: ingest_url.to_string(),
    };

    match handlers::knowledge_ingest_handler(
        axum::extract::State(app_state.clone()),
        Query(main::types::DebugParams::default()),
        Json(ingest_payload),
    )
    .await
    {
        Ok(Json(response)) => {
            info!(
                "Ingestion successful. Stored {} new FAQs.",
                response.result.ingested_faqs
            );
            if response.result.ingested_faqs == 0 {
                info!("Content may be unchanged from a previous run. Continuing...");
            }
        }
        Err(e) => {
            anyhow::bail!("Knowledge ingestion failed: {:?}. Please ensure your AI provider is running and configured in .env", e);
        }
    }

    // --- 3. Embed New FAQs ---
    info!("--- Starting Embedding for New FAQs ---");
    // --- 3. Embed New Documents ---
    info!("--- Starting Embedding for New Documents ---");
    // This will find all documents without an embedding and process them.
    let embed_payload = EmbedNewRequest { limit: Some(100) };

    match handlers::embed_new_handler(
        axum::extract::State(app_state.clone()),
        Query(main::types::DebugParams::default()),
        Json(embed_payload),
    )
    .await
    {
        Ok(_) => {
            info!("Embedding request completed successfully.");
        }
        Err(e) => {
            anyhow::bail!("Document embedding failed: {:?}", e);
        }
    }

    // --- 4. Ask Questions using RAG ---
    let question1 = "หากลูกค้าจ่ายบิลล่วงหน้าไม่เต็มบิล แต่มูลค่ามากกว่า 100 บาท จะได้รับสิทธิ์ไหม";
    let answer1 = ask_question(app_state.clone(), question1, None).await?;

    let question2 = "ทำยังไงถึงจะได้เทสล่า";
    let instruction2 = "สรุปเงื่อนไขการรับสิทธิ์ลุ้นเทสล่า";
    let answer2 = ask_question(app_state.clone(), question2, Some(instruction2)).await?;

    let question3 = "ถ้าใช้ True App เวอร์ชันเก่าอยู่ จะได้สิทธิ์ไหม?";
    let answer3 = ask_question(app_state.clone(), question3, None).await?;

    // --- 5. Print Final Results ---
    println!("\n\n✅ Knowledge RAG Workflow Complete!");
    println!("========================================");
    println!("❓ Question 1: {question1}");
    println!("💡 Answer 1:\n---\n{answer1}\n---");
    println!("\n========================================");
    println!("❓ Question 2: {question2}");
    println!("💡 Answer 2:\n---\n{answer2}\n---");
    println!("\n========================================");
    println!("❓ Question 3: {question3}");
    println!("💡 Answer 3:\n---\n{answer3}\n---");

    Ok(())
}
