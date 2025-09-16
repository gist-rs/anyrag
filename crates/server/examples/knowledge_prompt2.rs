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
//! `RUST_LOG=info cargo run -p anyrag-server --example knowledge_prompt2`

use anyhow::{bail, Result};
use anyrag_server::{
    auth::middleware::AuthenticatedUser,
    config,
    handlers::{self, EmbedNewRequest, IngestWebRequest, SearchRequest},
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

    let db_path = "db/anyrag-thai.db";
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
    let ingest_url = "https://www.gpf.or.th/thai2019/10contact/main.php?page=7&menu=askfreq&lang=th&size=n&pattern=n";
    let ingest_payload = IngestWebRequest {
        url: ingest_url.to_string(),
    };

    match handlers::ingest_web_handler(
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
    // let question1 = "GPF Point แจกถึงวันไหน";
    // let _answer1 = ask_question(app_state.clone(), auth_user.clone(), question1, None).await?;

    // let question2 = "สร้าง My GPF ได้รับที่คะแนน";
    // let _answer2 = ask_question(app_state.clone(), auth_user.clone(), question2, None).await?;

    // let question3 = "สร้าง My GPF ได้รับที่คะแนน";
    // let instruction3 = "สรุปเงื่อนไขการรับ GPF point, and start the answer with `สรุปเงื่อนไขได้ว่า`";
    // let _answer3 = ask_question(
    //     app_state.clone(),
    //     auth_user.clone(),
    //     question3,
    //     Some(instruction3),
    // )
    // .await?;

    let question4 = "มีเงิน 2 หมื่นออมต่อได้มั้ย";
    let answer4 = ask_question(app_state.clone(), auth_user, question4, None).await?;

    // --- 5. Print Final Results ---

    println!("\n\n✅ Knowledge RAG Workflow Complete!");
    println!("========================================");
    // println!("❓ Question 1: {question1}");
    // println!("💡 Answer 1:\n---\n{answer1}\n---");
    // println!("\n========================================");
    // println!("❓ Question 2: {question2}");
    // println!("💡 Answer 2:\n---\n{answer2}\n---");
    // println!("\n========================================");
    // println!("❓ Question 3: {question3}");
    // println!("💡 Answer 3:\n---\n{answer3}\n---");
    // println!("\n========================================");
    println!("❓ Question 4: {question4}");
    println!("💡 Answer 4:\n---\n{answer4}\n---");

    // assert!(_answer3.trim_matches('"').starts_with("สรุปเงื่อนไขได้ว่า"));
    assert!(answer4.trim_matches('"').contains("35,000"));

    Ok(())
    /* Expect
    ✅ Knowledge RAG Workflow Complete!
    ========================================
    ❓ Question 1: GPF Point แจกถึงวันไหน
    💡 Answer 1:
    ---
    "สมาชิกสามารถสร้าง My GPF หรือ My Twin1 หรือ My Twin 2 และรับ GPF Point รวม 50 คะแนนต่อคน ภายในวันที่ 31 ธันวาคม 2567 หรือตามเงื่อนไขที่ กบข. กำหนด"
    ---

    ========================================
    ❓ Question 2: สร้าง My GPF ได้รับที่คะแนน
    💡 Answer 2:
    ---
    "สร้าง My GPF รับ 10 คะแนนต่อคน"
    ---

    ========================================
    ❓ Question 3: สร้าง My GPF ได้รับที่คะแนน
    💡 Answer 3:
    ---
    "สมาชิกจะได้รับ GPF Point รวม 50 คะแนนต่อคน โดยแบ่งเป็น:
    - สร้าง My GPF: รับ 10 คะแนน
    - สร้าง My Twin 1: รับ 20 คะแนน
    - สร้าง My Twin 2: รับ 20 คะแนน

    เงื่อนไขนี้มีผลถึงวันที่ 31 ธันวาคม 2567 หรือตามที่ กบข. กำหนด"
    ---

    ========================================
    ❓ Question 4: มีเงิน 2 หมื่นออมต่อได้มั้ย
    💡 Answer 4:
    ---
    "ไม่ได้ เพราะยอดเงินในบัญชีไม่ต่ำกว่า 35,000 บาท ณ วันที่ยื่นคำขอ โดยมีเอกสารหลักฐานถูกต้องครบถ้วน"
    ---
    */
}
