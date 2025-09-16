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
//! - A valid `.env` file in the workspace root (`anyrag/`) with credentials
//!   for a running AI provider (e.g., a local Ollama server).
//! - An internet connection to fetch the URL.
//!
//! # Usage
//!
//! From the workspace root (`anyrag/`):
//! `RUST_LOG=info cargo run -p anyrag-server --example knowledge_prompt`

use anyhow::{bail, Result};
use anyrag_server::{
    auth::middleware::AuthenticatedUser,
    config,
    handlers::ingest::web::{ingest_web_handler, IngestWebRequest},
    handlers::{self, EmbedNewRequest, SearchRequest},
    state::{self, AppState},
    types::DebugParams,
};
use axum::{extract::Query, Json};
use core_access::get_or_create_user;
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
    user: AuthenticatedUser,
    query: &str,
    instruction: Option<&str>,
) -> Result<String> {
    info!("--- Asking Question: '{}' ---", query);

    let payload = SearchRequest {
        query: query.to_string(),
        instruction: instruction.map(String::from),
        limit: Some(5), // How many KB entries to use for context
        mode: Default::default(),
        use_knowledge_graph: Some(true),
    };

    let result = handlers::knowledge_search_handler(
        axum::extract::State(app_state),
        user,
        Query(DebugParams::default()),
        Json(payload),
    )
    .await;

    match result {
        Ok(Json(response)) => Ok(response.result.text.to_string()),
        Err(e) => anyhow::bail!("Error occurred while asking question: {:?}", e),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // --- 1. Setup ---
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    dotenvy::from_path(".env").ok();
    info!("Environment variables loaded.");

    let db_path = "db/anyrag.db";
    cleanup_db(db_path).await?;
    // This is set so the AppState builder uses the correct path.
    std::env::set_var("DB_URL", db_path);

    // When running examples from the workspace root, we need to point to the config file.
    let config_path = "crates/server/config.yml";
    let fallback_path = "crates/server/config.gemini.yml";
    let final_config_path = if std::path::Path::new(config_path).exists() {
        config_path
    } else if std::path::Path::new(fallback_path).exists() {
        info!("'{config_path}' not found, using template '{fallback_path}' as a fallback.");
        fallback_path
    } else {
        bail!("Configuration file not found. Please copy '{fallback_path}' to '{config_path}' to run this example.");
    };
    info!(
        "Loading configuration for example from: {}",
        final_config_path
    );

    let config = config::get_config(Some(final_config_path))
        .unwrap_or_else(|e| panic!("Failed to load configuration: {e}"));
    let app_state = state::build_app_state(config).await?;
    info!("Application state built successfully.");

    // Create a user for this example run. In a real app, this would come from a JWT.
    let user = get_or_create_user(
        &app_state.sqlite_provider.db,
        "example-user@anyrag.com",
        None,
    )
    .await?;
    let auth_user = AuthenticatedUser(user);
    info!("Simulating requests for user: {}", auth_user.0.id);

    sleep(Duration::from_millis(100)).await;

    // --- 2. Ingest Knowledge ---
    info!("--- Starting Knowledge Ingestion ---");
    let ingest_url = "https://www.true.th/betterliv/support/true-app-mega-campaign";
    let ingest_payload = IngestWebRequest {
        url: ingest_url.to_string(),
    };

    match ingest_web_handler(
        axum::extract::State(app_state.clone()),
        auth_user.clone(),
        Query(DebugParams::default()),
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

    // --- 3. Embed New Documents ---
    info!("--- Starting Embedding for New Documents ---");
    // This will find all documents without an embedding and process them.
    let embed_payload = EmbedNewRequest { limit: Some(100) };

    match handlers::embed_new_handler(
        axum::extract::State(app_state.clone()),
        Query(DebugParams::default()),
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
    let answer1 = ask_question(app_state.clone(), auth_user.clone(), question1, None).await?;

    let question2 = "ทำยังไงถึงจะได้เทสล่า";
    let instruction2 = "สรุปเงื่อนไขการรับสิทธิ์ลุ้นเทสล่า";
    let answer2 = ask_question(
        app_state.clone(),
        auth_user.clone(),
        question2,
        Some(instruction2),
    )
    .await?;

    let question3 = "ถ้าใช้ True App เวอร์ชันเก่าอยู่ จะได้สิทธิ์ไหม?";
    let answer3 = ask_question(app_state.clone(), auth_user, question3, None).await?;

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
