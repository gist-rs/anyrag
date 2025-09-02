//! Example: End-to-end Google Sheet FAQ RAG workflow for date-sensitive questions.
//!
//! This example demonstrates the full workflow for sheet-based FAQs with time constraints:
//! 1.  It ingests Q&A pairs from a public Google Sheet, including `start_at` and `end_at` columns.
//! 2.  It generates vector embeddings for these new FAQs.
//! 3.  It uses the RAG pattern (`/search/knowledge`) to ask a question ("Hobby?")
//!     that has different answers depending on the current date.
//!
//! # Prerequisites
//!
//! - A valid `.env` file in the workspace root (`anyrag/`) with credentials
//!   for a running AI provider.
//! - An internet connection to fetch the Google Sheet.
//!
//! # Usage
//!
//! From the workspace root (`anyrag/`):
//! `RUST_LOG=info cargo run -p anyrag-server --example sheet_faq_date_sensitive`

use anyhow::{bail, Result};
use anyrag_server::{
    auth::middleware::AuthenticatedUser,
    config,
    handlers::{self, EmbedNewRequest, IngestSheetFaqRequest, SearchRequest},
    state,
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
    dotenvy::from_path(".env").ok();
    info!("Environment variables loaded.");

    let db_path = "db/anyrag_sheet_faq_date.db";
    cleanup_db(db_path).await?;
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

    let config =
        config::get_config(Some(final_config_path)).expect("Failed to load configuration.");
    let app_state = state::build_app_state(config).await?;
    info!("Application state built successfully.");

    // Create a user for this example run.
    let user = get_or_create_user(
        &app_state.sqlite_provider.db,
        "example-user-sheet-faq@anyrag.com",
    )
    .await?;
    let auth_user = AuthenticatedUser(user);
    info!("Simulating requests for user: {}", auth_user.0.id);

    sleep(Duration::from_millis(100)).await;

    // --- 2. Ingest FAQs from Google Sheet ---
    info!("--- Starting Google Sheet FAQ Ingestion ---");
    let sheet_url = "https://docs.google.com/spreadsheets/d/1Upsr6r6ufkYougDFVBQOQNgNf9Syrwv2CTNhFbVNu2w/edit?usp=sharing";
    let ingest_payload = IngestSheetFaqRequest {
        url: sheet_url.to_string(),
        gid: Some(856666263.to_string()),
        skip_header: true,
    };

    match handlers::ingest_sheet_faq_handler(
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
                bail!("No FAQs were ingested. The sheet might be empty or already processed.");
            }
        }
        Err(e) => {
            bail!("Sheet FAQ ingestion failed: {:?}", e);
        }
    }

    // --- 3. Embed New FAQs ---
    info!("--- Starting Embedding for New Documents ---");
    let embed_payload = EmbedNewRequest { limit: Some(100) };

    match handlers::embed_new_handler(
        axum::extract::State(app_state.clone()),
        Query(DebugParams::default()),
        Json(embed_payload),
    )
    .await
    {
        Ok(_) => info!("Embedding request completed successfully."),
        Err(e) => bail!("Document embedding failed: {:?}", e),
    }

    // --- 4. Ask a Question using RAG ---
    info!("--- Asking Date-Sensitive Question against Sheet Knowledge ---");
    let question = "What is the current hobby?";
    let search_payload = SearchRequest {
        query: question.to_string(),
        instruction: None,
        limit: Some(3),
        mode: Default::default(),
        use_knowledge_graph: Some(false),
    };

    let final_answer = match handlers::knowledge_search_handler(
        axum::extract::State(app_state.clone()),
        auth_user,
        Query(DebugParams::default()),
        Json(search_payload),
    )
    .await
    {
        Ok(Json(response)) => response.result.text,
        Err(e) => bail!("Error occurred while asking question: {:?}", e),
    };

    // --- 5. Print Final Results ---
    println!("\n\n✅ Google Sheet Date-Sensitive FAQ RAG Workflow Complete!");
    println!("===========================================================");
    println!("❓ Question: {question}");
    println!("💡 Answer:\n---\n{final_answer}\n---");

    Ok(())
}
