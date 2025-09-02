//! Example: Demonstrating Knowledge Graph Precision (Harry Potter Edition)
//!
//! This example showcases how the Knowledge Graph can provide precise, time-sensitive
//! answers that override more generic information from the standard knowledge base.
//!
//! # Workflow:
//! 1.  A generic, always-true fact about Harry Potter is seeded into the main database.
//! 2.  A timeline of Harry's roles (past, present, and future) is seeded into the in-memory Knowledge Graph.
//! 3.  The same question, "What is Harry Potter's current role?", is asked twice:
//!     a. First, with `use_knowledge_graph: false`.
//!     b. Second, with `use_knowledge_graph: true`.
//! 4.  The results are printed, clearly showing that the Knowledge Graph provides the correct, time-aware answer.
//!
//! # Prerequisites
//!
//! - A valid `.env` file in `crates/server` with credentials for a running AI provider.
//!
//! # Usage
//!
//! From the workspace root (`anyrag/`):
//! `RUST_LOG=info cargo run -p anyrag-server --example knowledge_graph_precision`

use anyhow::Result;
use anyrag_server::{
    auth::middleware::AuthenticatedUser,
    config,
    handlers::{self, SearchRequest},
    state::{self, AppState},
    types::DebugParams,
};
use axum::{extract::Query, Json};
use chrono::{Duration, Utc};
use core_access::get_or_create_user;
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

/// A helper to call the RAG endpoint.
async fn ask_question(
    app_state: AppState,
    user: AuthenticatedUser,
    query: &str,
    use_kg: bool,
) -> Result<String> {
    info!(
        "--- Asking Question: '{}' (using Knowledge Graph: {}) ---",
        query, use_kg
    );

    let payload = SearchRequest {
        query: query.to_string(),
        instruction: None,
        limit: Some(5),
        mode: Default::default(),
        use_knowledge_graph: Some(use_kg),
    };

    let result = handlers::knowledge_search_handler(
        axum::extract::State(app_state),
        user,
        Query(DebugParams::default()),
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

    let db_path = "db/anyrag_kg_harry_potter.db";
    cleanup_db(db_path).await?;
    std::env::set_var("DB_URL", db_path);

    let config = config::get_config().expect("Failed to load configuration. Is .env present?");
    let app_state = state::build_app_state(config).await?;
    info!("Application state built successfully.");

    // Create a user for this example run.
    let user =
        get_or_create_user(&app_state.sqlite_provider.db, "example-user-kg@anyrag.com").await?;
    let auth_user = AuthenticatedUser(user);
    info!("Simulating requests for user: {}", auth_user.0.id);

    sleep(StdDuration::from_millis(100)).await;

    // --- 2. Define Scenario Data ---
    let subject = "Harry_Potter";
    let question = "What is Harry Potter's current role?";

    let generic_answer = "Harry Potter is a famous wizard known for defeating Voldemort.";
    let past_role = "Student at Hogwarts";
    let present_role = "Head of Magical Law Enforcement";
    let future_role = "Retired Auror";

    // --- 3. Seed Databases with Conflicting Information ---
    // A. Seed the regular KB with the generic, non-time-sensitive answer.
    info!("Seeding regular KB with generic fact...");
    let conn = app_state.sqlite_provider.db.connect()?;
    let document_id = "doc_wizarding_world";
    // First, create the parent document.
    conn.execute(
        "INSERT OR IGNORE INTO documents (id, source_url, title, content) VALUES (?, ?, ?, ?)",
        turso::params![
            document_id,
            "wizarding_world.txt",
            "Wizarding World Facts",
            generic_answer
        ],
    )
    .await?;
    // Now, insert the FAQ item associated with that document.
    conn.execute(
        "INSERT INTO faq_items (document_id, question, answer) VALUES (?, ?, ?)",
        turso::params![document_id, question, generic_answer],
    )
    .await?;
    info!("Regular KB seeded.");

    // B. Seed the Knowledge Graph with the precise, time-sensitive roles.
    info!("Seeding Knowledge Graph with time-sensitive facts...");
    let now = Utc::now();
    {
        let mut kg = app_state
            .knowledge_graph
            .write()
            .expect("Failed to get write lock on KG");
        // Past fact: Ended yesterday
        kg.add_fact(
            subject,
            "role",
            past_role,
            now - Duration::days(365 * 7), // ~7 years ago
            now - Duration::days(1),
        )?;
        // Present fact: Active now
        kg.add_fact(
            subject,
            "role",
            present_role,
            now - Duration::days(1),
            now + Duration::days(365),
        )?;
        // Future fact: Starts next year
        kg.add_fact(
            subject,
            "role",
            future_role,
            now + Duration::days(365),
            now + Duration::days(365 * 10),
        )?;
    }
    info!("Knowledge Graph seeded.");

    // --- 4. Ask Questions ---
    // The regular KB only has one entry, so we don't need to run embeddings for this demo.
    let answer_without_kg =
        ask_question(app_state.clone(), auth_user.clone(), question, false).await?;
    let answer_with_kg = ask_question(app_state.clone(), auth_user, subject, true).await?;

    // --- 5. Print Final Results ---
    println!("\n\n✅ Knowledge Graph Precision Demo Complete!");
    println!("======================================================");
    println!("Scenario: We have two sources of information about Harry Potter.");
    println!("- The General KB: `{generic_answer}` (Always true, but not specific)");
    println!("- The Knowledge Graph holds a timeline of his roles:");
    println!("  - Past: {past_role}");
    println!("  - Present: {present_role}");
    println!("  - Future: {future_role}");
    println!("---");
    println!("\n❓ Question: {question}");
    println!("---");
    println!("\n💡 Answer (Without Knowledge Graph):");
    println!("   -> The AI uses the generic fact from the database.");
    println!("   -> {answer_without_kg}");
    println!("\n💡 Answer (WITH Knowledge Graph):");
    println!("   -> The AI is given the definitive, time-sensitive fact and prioritizes it.");
    println!("   -> {answer_with_kg}");
    println!("\n======================================================");

    Ok(())
}
