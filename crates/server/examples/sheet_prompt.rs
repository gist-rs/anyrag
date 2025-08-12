//! Example: Ingesting and prompting a Google Sheet.
//!
//! This example demonstrates the full workflow of providing a Google Sheet URL
//! within a prompt. It will:
//! 1. Load configuration from your `.env` file.
//! 2. Connect to the necessary services (AI provider, local SQLite DB).
//! 3. Ingest the data from a public Google Sheet into a local SQLite table.
//! 4. Generate and execute a SQL query based on your prompt against that table.
//! 5. Format the result using the AI provider.
//!
//! # Prerequisites
//!
//! - A valid `.env` file in the `crates/server` directory.
//! - A running AI provider (e.g., a local Ollama server).
//!
//! # Usage
//!
//! From the workspace root (`anyrag/`):
//! `cargo run -p anyrag-server --example sheet_prompt`

// This is a common pattern for creating examples for a binary crate.
// It includes the binary's main source file as a module.
#[path = "../src/main.rs"]
mod main;

use axum::Json;
use std::fs;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging and load environment variables.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    dotenvy::dotenv().ok();
    tracing::info!("Environment variables loaded.");

    // Clean up previous database files to ensure fresh ingestion.
    for path in ["anyrag.db", "anyrag.db-wal"] {
        if fs::metadata(path).is_ok() {
            fs::remove_file(path)?;
            tracing::info!("Removed existing database file: {}", path);
        }
    }

    // Clean up previous database file to ensure fresh ingestion.
    let db_path = "crates/server/anyrag.db";
    if fs::metadata(db_path).is_ok() {
        fs::remove_file(db_path)?;
        tracing::info!("Removed existing database file: {}", db_path);
    }

    // Build the application state, which initializes providers.
    let config =
        main::config::get_config().expect("Failed to load configuration. Is .env present?");
    let app_state = main::state::build_app_state(config).await?;
    tracing::info!("Application state built successfully.");

    // Define the prompt with a public Google Sheet URL.
    // This sheet contains sample employee data.
    let sheet_url =
        "https://docs.google.com/spreadsheets/d/1Upsr6r6ufkYougDFVBQOQNgNf9Syrwv2CTNhFbVNu2w/edit";
    let prompt = format!("What was yesterday's email about? {sheet_url}");
    let instruction = "Please answer in a complete, natural sentence.";
    tracing::info!("Prompt: {}", prompt);

    // Create the JSON payload, simulating an API request.
    let payload = serde_json::json!({
        "prompt": prompt,
        "instruction": instruction,
    });

    // Directly call the handler function with the app state and payload.
    tracing::info!("Executing prompt handler...");
    let result =
        main::handlers::prompt_handler(axum::extract::State(app_state), Json(payload)).await;

    // Print the final result.
    match result {
        Ok(Json(response)) => {
            println!("\n✅ Success!");
            println!("Final formatted response:\n---\n{}\n---", response.result);
        }
        Err(e) => {
            // The AppError has a custom IntoResponse implementation, but for a CLI,
            // we'll just print its debug representation.
            eprintln!("\n❌ An error occurred: {e:?}");
        }
    }

    Ok(())
}
