//! Example: Demonstrating Knowledge Graph Precision
//!
//! This example showcases how the Knowledge Graph can override or augment
//! information from the general knowledge base to provide more precise,
//! definitive answers.
//!
//! # Workflow:
//! 1.  A generic, less-specific answer is seeded into the main `faq_kb` database table.
//! 2.  A precise, time-sensitive, and correct answer is seeded into the in-memory Knowledge Graph.
//! 3.  The same question is asked twice to the RAG endpoint (`/search/knowledge`):
//!     a. First, with `use_knowledge_graph: false`.
//!     b. Second, with `use_knowledge_graph: true`.
//! 4.  The results are printed, demonstrating how the Knowledge Graph provides a more accurate answer.
//!
//! # Prerequisites
//!
//! - A valid `.env` file in the `crates/server` directory with credentials for a running AI provider.
//!
//! # Usage
//!
//! From the workspace root (`anyrag/`):
//! `RUST_LOG=info cargo run -p anyrag-server --example knowledge_graph_precision`

// Include the binary's main source file to access its components.
#[path = "../src/main.rs"]
mod main;

use anyhow::Result;
use axum::{extract::Query, Json};
use chrono::{Duration, Utc};
use main::{
    handlers::{self, SearchRequest},
    state::AppState,
};
use std::{fs, time::Duration as StdDuration};
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
async fn ask_question(app_state: AppState, query: &str, use_kg: bool) -> Result<String> {
    info!(
        "--- Asking Question: '{}' (using Knowledge Graph: {}) ---",
        query, use_kg
    );

    let payload = SearchRequest {
        query: query.to_string(),
        instruction: None,
        limit: Some(5),
        mode: anyrag::SearchMode::LlmReRank, // Not critical for this example
        use_knowledge_graph: Some(use_kg),
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

    let db_path = "db/anyrag_kg_precision.db";
    cleanup_db(db_path).await?;
    std::env::set_var("DB_URL", db_path);

    let config =
        main::config::get_config().expect("Failed to load configuration. Is .env present?");
    let app_state = main::state::build_app_state(config).await?;
    info!("Application state built successfully.");
    sleep(StdDuration::from_millis(100)).await;

    // --- 2. Seed Conflicting Information ---
    let subject = "SuperWidget X500";
    let question = "What is the power source for the SuperWidget X500?";
    let generic_answer = "The SuperWidget X500 is powered by a standard rechargeable battery pack.";
    let precise_answer = "The primary power source is the TX-300 Solar Array.";

    // A. Seed the regular KB with the generic answer.
    info!("Seeding regular knowledge base with generic answer...");
    let conn = app_state.sqlite_provider.db.connect()?;
    anyrag::ingest::knowledge::create_kb_tables_if_not_exists(&conn).await?;
    conn.execute(
        "INSERT INTO faq_kb (question, answer, source_url, is_explicit, content_hash, last_modified) VALUES (?, ?, ?, ?, ?, ?)",
        turso::params![
            question,
            generic_answer,
            "generic_manual.txt",
            true,
            "hash_generic",
            Utc::now().to_rfc3339()
        ],
    ).await?;
    info!("Regular KB seeded.");

    // B. Seed the Knowledge Graph with the precise, correct answer.
    info!("Seeding Knowledge Graph with precise answer...");
    {
        let mut kg = app_state
            .knowledge_graph
            .write()
            .expect("Failed to get write lock on KG");
        kg.add_fact(
            subject,
            "role", // Using a consistent predicate for this example set
            precise_answer,
            Utc::now() - Duration::days(1),
            Utc::now() + Duration::days(1),
        )?;
    }
    info!("Knowledge Graph seeded.");

    // --- 3. Ask Questions and Compare ---

    // A. Ask WITHOUT using the Knowledge Graph
    // For the query, we use the full question to find the entry in the faq_kb.
    let answer_without_kg = ask_question(app_state.clone(), question, false).await?;

    // B. Ask WITH using the Knowledge Graph
    // For this query, we use the clean "subject" to ensure we get a direct hit from the KG.
    let answer_with_kg = ask_question(app_state.clone(), subject, true).await?;

    // --- 4. Print Final Results ---
    println!("\n\n✅ Knowledge Graph Precision Demo Complete!");
    println!("======================================================");
    println!("Scenario: The database contains a generic answer, but the Knowledge Graph holds a precise, definitive fact.");
    println!("\n---");
    println!("❓ Question: {question}");
    println!("---");
    println!("\n💡 Answer (Without Knowledge Graph):");
    println!("   -> {answer_without_kg}");
    println!("\n💡 Answer (WITH Knowledge Graph):");
    println!("   -> {answer_with_kg}");
    println!("\n======================================================");

    Ok(())
}
