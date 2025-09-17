//! Example: Backfill Metadata for Existing Documents
//!
//! This script is a one-time utility to process all existing documents in the
//! database and generate structured metadata (Entities and Keyphrases) for them.
//! This is useful after an initial data dump to enrich the content for hybrid search.
//!
//! # Workflow:
//! 1.  Connects to the main application database specified in the `.env` file.
//! 2.  Fetches all documents from the `documents` table.
//! 3.  For each document, it calls the AI provider using the configured
//!     `knowledge_metadata_extraction` prompt.
//! 4.  It then stores the newly extracted metadata, linking it to the original document.
//!
//! # Prerequisites
//!
//! - A valid `.env` file in the workspace root (`anyrag/`) with a `DB_URL`
//!   and credentials for a running AI provider.
//!
//! # Usage
//!
//! From the workspace root (`anyrag/`):
//! `RUST_LOG=info cargo run -p anyrag-server --example backfill_metadata`

use anyhow::{bail, Result};
use anyrag_server::{config, state};
use anyrag_web::extract_and_store_metadata;
use futures::stream::{self, StreamExt};
use std::env;
use tracing::info;
use tracing_subscriber::EnvFilter;

const CONCURRENT_REQUESTS: usize = 5; // Number of parallel requests to the AI provider

#[tokio::main]
async fn main() -> Result<()> {
    // --- 1. Setup ---
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    dotenvy::from_path(".env").ok();
    info!("Environment variables loaded.");

    let config_path = "crates/server/config.yml";
    let fallback_path = "crates/server/config.local.yml";
    let final_config_path = if std::path::Path::new(config_path).exists() {
        config_path
    } else if std::path::Path::new(fallback_path).exists() {
        info!("'config.yml' not found, using template '{fallback_path}' as a fallback.");
        fallback_path
    } else {
        bail!("Configuration file not found. Please copy '{fallback_path}' to '{config_path}' to run this example.");
    };

    let db_url = env::var("DB_URL").expect("DB_URL must be set in your .env file");
    info!("Starting metadata backfill for database: {}", db_url);

    let config =
        config::get_config(Some(final_config_path)).expect("Failed to load configuration.");
    let app_state = state::build_app_state(config).await?;
    info!("Application state built successfully.");

    // --- 2. Fetch Documents to Process ---
    let conn = app_state.sqlite_provider.db.connect()?;

    // To make the script idempotent, we first delete all existing keyphrase and category metadata.
    // This ensures that if the script is run again, it will re-process everything
    // with the latest prompt and model logic.
    info!("Clearing existing 'KEYPHRASE' and 'CATEGORY' metadata to ensure a clean backfill...");
    conn.execute(
        "DELETE FROM content_metadata WHERE metadata_type = 'KEYPHRASE' OR metadata_type = 'CATEGORY'",
        (),
    )
    .await?;

    let mut rows = conn
        .query("SELECT id, owner_id, content FROM documents", ())
        .await?;

    let mut docs_to_process = Vec::new();
    while let Some(row) = rows.next().await? {
        let doc_id: String = row.get(0)?;
        let owner_id: Option<String> = row.get(1)?;
        let content: String = row.get(2)?;
        docs_to_process.push((doc_id, owner_id, content));
    }

    let total_docs = docs_to_process.len();
    if total_docs == 0 {
        info!("No documents found in the database. Exiting.");
        return Ok(());
    }
    info!(
        "Found {} documents to process for metadata extraction.",
        total_docs
    );

    // --- 3. Get AI Provider and Prompts ---
    let meta_task_config = app_state
        .tasks
        .get("knowledge_metadata_extraction")
        .ok_or_else(|| anyhow::anyhow!("Task 'knowledge_metadata_extraction' not found"))?;

    let ai_provider = app_state
        .ai_providers
        .get(&meta_task_config.provider)
        .ok_or_else(|| anyhow::anyhow!("Provider '{}' not found", meta_task_config.provider))?
        .clone();
    let system_prompt = meta_task_config.system_prompt.clone();

    // --- 4. Process Documents Concurrently ---
    let mut successful_count = 0;
    let mut failed_count = 0;

    let mut stream = stream::iter(docs_to_process)
        .map(|(doc_id, owner_id, content)| {
            let provider = ai_provider.clone();
            let prompt = system_prompt.clone();
            let conn = app_state.sqlite_provider.db.connect().unwrap();
            async move {
                info!("Processing document ID: {}", doc_id);
                let result = extract_and_store_metadata(
                    &conn,
                    provider.as_ref(),
                    &doc_id,
                    owner_id.as_deref(),
                    &content,
                    &prompt,
                )
                .await;
                (doc_id, result)
            }
        })
        .buffer_unordered(CONCURRENT_REQUESTS);

    while let Some((doc_id, result)) = stream.next().await {
        match result {
            Ok(_) => {
                // info!("Successfully extracted metadata for doc_id: {}", doc_id);
                successful_count += 1;
            }
            Err(e) => {
                info!(
                    "Failed to extract metadata for doc_id: {}. Error: {}",
                    doc_id, e
                );
                failed_count += 1;
            }
        }
    }

    // --- 5. Print Final Summary ---
    println!("\n\n✅ Metadata Backfill Complete!");
    println!("========================================");
    println!("Total Documents Processed: {total_docs}");
    println!("          Successful: {successful_count}");
    println!("              Failed: {failed_count}");
    println!("========================================");

    Ok(())
}
