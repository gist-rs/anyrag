//! Example: End-to-end BigQuery prompt execution.
//!
//! This example demonstrates the new, dynamic BigQuery client functionality.
//! It shows how to execute a Text-to-SQL prompt against a BigQuery public
//! dataset by providing the `project_id` in the request payload.
//!
//! # Workflow:
//! 1.  Loads configuration, ensuring `BIGQUERY_PROJECT_ID` is set in the `.env` file.
//! 2.  Builds the standard `AppState`, which defaults to using SQLite.
//! 3.  Constructs a payload for the `prompt_handler` that includes the user's
//!     prompt, a BigQuery table name, and the `project_id`.
//! 4.  Calls the `prompt_handler` directly. The handler detects the `project_id`
//!     and dynamically creates a BigQuery client for this specific request.
//! 5.  Prints the final, formatted answer from the AI.
//!
//! # Prerequisites
//!
//! - A valid `.env` file in the workspace root (`anyrag/`) with credentials
//!   for a running AI provider and a valid `BIGQUERY_PROJECT_ID`.
//! - A Google Cloud account with the necessary permissions. You must be authenticated
//!   locally using `gcloud auth application-default login`.
//!
//! # Usage
//!
//! From the workspace root (`anyrag/`):
//! `RUST_LOG=info cargo run -p anyrag-server --example bigquery_prompt`

use anyhow::{bail, Result};
use anyrag_server::{config, handlers, state, types::DebugParams};
use axum::{extract::Query, Json};
use serde_json::json;
use std::env;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // --- 1. Setup ---
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    // Load .env from the workspace root.
    dotenvy::from_path(".env").ok();
    info!("Environment variables loaded.");

    // --- 2. Load Config & Explicitly Get Project ID ---
    // This is the project that will be billed for the query.
    let project_id = match env::var("BIGQUERY_PROJECT_ID") {
        Ok(id) if !id.is_empty() => id,
        _ => {
            bail!("BIGQUERY_PROJECT_ID is not set in the .env file. This example requires it.")
        }
    };

    let config_path = "crates/server/config.yml";
    let fallback_path = "crates/server/config.gemini.yml";
    let final_config_path = if std::path::Path::new(config_path).exists() {
        config_path
    } else if std::path::Path::new(fallback_path).exists() {
        info!("'config.yml' not found, using template '{fallback_path}' as a fallback.");
        fallback_path
    } else {
        bail!("Configuration file not found. Please copy '{fallback_path}' to '{config_path}' to run this example.");
    };

    let config =
        config::get_config(Some(final_config_path)).expect("Failed to load configuration.");
    // The AppState will initialize with a default SQLite client, as designed.
    let app_state = state::build_app_state(config).await?;
    info!("Application state built successfully (defaulting to SQLite).");

    // --- 3. Construct the BigQuery API Payload ---
    let question = "How many distinct corpuses are in the shakespeare dataset?";
    let table_name = "bigquery-public-data.samples.shakespeare";

    let prompt_payload = json!({
        "prompt": question,
        "table_name": table_name,
        "project_id": project_id, // This is the key that triggers the dynamic BigQuery client
        "instruction": "Answer with only the number.",
    });

    info!(
        payload = ?prompt_payload,
        "Sending request to prompt_handler with BigQuery project_id."
    );

    // --- 4. Call the main prompt handler ---
    let final_answer = match handlers::prompt_handler(
        axum::extract::State(app_state.clone()),
        // Enable the debug flag to get more information back
        Query(DebugParams { debug: Some(true) }),
        Json(prompt_payload),
    )
    .await
    {
        Ok(Json(response)) => {
            println!(
                "\n--- Debug Info --- \n{:#?}\n------------------",
                response.debug
            );
            response.result.text
        }
        Err(e) => anyhow::bail!("Error occurred while asking question: {:?}", e),
    };

    // --- 5. Print Final Results ---
    println!("\n\n✅ BigQuery Dynamic Client Workflow Complete!");
    println!("========================================");
    println!("❓ Question: {question}");
    println!("💡 Answer:\n---\n{final_answer}\n---");

    Ok(())
}
