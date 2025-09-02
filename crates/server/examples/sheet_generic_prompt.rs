//! Example: End-to-end generic Google Sheet RAG workflow.
//!
//! This example demonstrates the full workflow for querying a generic sheet:
//! 1.  It dynamically ingests data from a public Google Sheet (the 'my-email' tab).
//! 2.  It uses the Text-to-SQL capabilities to answer a natural language question
//!     about the sheet's content.
//!
//! # Prerequisites
//!
//! - A valid `.env` file in the `crates/server` directory with credentials
//!   for a running AI provider.
//! - An internet connection to fetch the Google Sheet.
//!
//! # Usage
//!
//! From the workspace root (`anyrag/`):
//! `RUST_LOG=info cargo run -p anyrag-server --example sheet_generic_prompt`

use anyhow::Result;
use anyrag_server::{config, handlers, state, types::DebugParams};
use axum::{extract::Query, Json};
use serde_json::json;
use std::{fs, time::Duration};
use tokio::time::sleep;
use tracing::info;
use tracing_subscriber::EnvFilter;

/// Cleans up database files for a fresh run.
async fn cleanup_db(db_path: &str) -> Result<()> {
    for path_str in [db_path, &format!("{db_path}-wal")] {
        let path = std::path::Path::new(path_str);
        if path.exists() {
            fs::remove_file(path)?;
            info!("Removed existing database file: {}", path.display());
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // --- 1. Setup ---
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    dotenvy::dotenv().ok();
    info!("Environment variables loaded.");

    let db_path = "db/anyrag_sheet_generic.db";
    cleanup_db(db_path).await?;

    // Set DB_URL so the app state uses the same DB as the cleanup function.
    std::env::set_var("DB_URL", db_path);

    let config = config::get_config(None).expect("Failed to load configuration. Is .env present?");
    let app_state = state::build_app_state(config).await?;
    info!("Application state built successfully.");

    sleep(Duration::from_millis(100)).await;

    // --- 2. Ask a Question using the Sheet ---
    info!("--- Asking Question against Generic Sheet ---");
    // This URL points to the 'my-email' tab, which is the default (gid=0).
    // The prompt handler automatically ingests the default tab of any sheet URL it finds.
    let sheet_url = "https://docs.google.com/spreadsheets/d/1Upsr6r6ufkYougDFVBQOQNgNf9Syrwv2CTNhFbVNu2w/edit?usp=sharing";
    let question = "What was the email from Emily Brown about?";

    // The main `/prompt` endpoint is designed to automatically detect and ingest a sheet URL
    // when it's part of the prompt. We construct a payload that includes both the URL and the question.
    let prompt_payload = json!({
        "prompt": format!("Using the data from this sheet {}, answer the following question: {}", sheet_url, question),
        "instruction": "Summarize the email body.",
    });

    // --- 3. Call the main prompt handler ---
    let final_answer = match handlers::prompt_handler(
        axum::extract::State(app_state.clone()),
        Query(DebugParams::default()),
        Json(prompt_payload),
    )
    .await
    {
        Ok(Json(response)) => response.result.text,
        Err(e) => anyhow::bail!("Error occurred while asking question: {:?}", e),
    };

    // --- 4. Print Final Results ---
    println!("\n\n✅ Google Sheet Generic Query Workflow Complete!");
    println!("========================================");
    println!("❓ Question: {question}");
    println!("💡 Answer:\n---\n{final_answer}\n---");

    Ok(())
}
