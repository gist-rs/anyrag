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
use anyrag::constants;
use anyrag_server::{
    auth::middleware::AuthenticatedUser,
    config,
    handlers::ingest::web::{ingest_web_handler, IngestWebRequest},
    handlers::{self, EmbedNewRequest, SearchRequest},
    state::{self, AppState},
    types::DebugParams,
};
use axum::{extract::Query, Json};
use core_access::{get_or_create_user, GUEST_USER_IDENTIFIER};
use std::{fs, time::Duration};
use tokio::time::sleep;
use tracing::info;
use tracing_subscriber::EnvFilter;

/// Cleans up database files for a fresh run.
#[allow(dead_code)]
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
        db: None,
        query: query.to_string(),
        model: None,
        instruction: instruction.map(String::from),
        limit: Some(5), // How many KB entries to use for context
        mode: Default::default(),
        use_knowledge_graph: Some(true),
        context: None,
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
        Err(e) => anyhow::bail!("Error occurred while asking question: {e:?}"),
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

    let db_path = format!("{}/anyrag-thai.db", constants::DB_DIR);
    // cleanup_db(&db_path).await?;
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
    let user =
        get_or_create_user(&app_state.sqlite_provider.db, GUEST_USER_IDENTIFIER, None).await?;
    let auth_user = AuthenticatedUser(user);
    info!("Simulating requests for user: {}", auth_user.0.id);

    sleep(Duration::from_millis(100)).await;

    // --- 2. Ingest Knowledge ---
    info!("--- Starting Knowledge Ingestion ---");
    let ingest_url = "https://www.gpf.or.th/thai2019/10contact/main.php?page=7&menu=askfreq&lang=th&size=n&pattern=n";
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
                response.result.ingested_documents
            );
            if response.result.ingested_documents == 0 {
                info!("Content may be unchanged from a previous run. Continuing...");
            }
        }
        Err(e) => {
            anyhow::bail!("Knowledge ingestion failed: {e:?}. Please ensure your AI provider is running and configured in .env");
        }
    }

    // --- 2.1 Ingest More Knowledge ---
    info!("--- Starting Knowledge Ingestion ---");
    let ingest_url = "https://www.gpf.or.th/thai2019/About/main.php?page=chart&menu=statistic&lang=th&size=n&pattern=n";
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
                response.result.ingested_documents
            );
            if response.result.ingested_documents == 0 {
                info!("Content may be unchanged from a previous run. Continuing...");
            }
        }
        Err(e) => {
            anyhow::bail!("Knowledge ingestion failed: {e:?}. Please ensure your AI provider is running and configured in .env");
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
            anyhow::bail!("Document embedding failed: {e:?}");
        }
    }

    // --- 4. Ask Questions using RAG ---
    let question1 = "GPF Point แจกถึงวันไหน";
    let answer1 = ask_question(app_state.clone(), auth_user.clone(), question1, None).await?;

    let question2 = "สร้าง My GPF ได้รับที่คะแนน";
    let answer2 = ask_question(app_state.clone(), auth_user.clone(), question2, None).await?;

    // With custom instructions
    let question3 = "สร้าง My GPF ได้รับที่คะแนน";
    let instruction3 = "สรุปเงื่อนไขการรับ GPF point, and start the answer with `สรุปเงื่อนไขได้ว่า`";
    let answer3 = ask_question(
        app_state.clone(),
        auth_user.clone(),
        question3,
        Some(instruction3),
    )
    .await?;

    let question4 = "มีเงิน 2 หมื่นออมต่อได้มั้ย";
    let answer4 = ask_question(app_state.clone(), auth_user.clone(), question4, None).await?;

    // Prove 2 ingests
    let question5 = "จำนวนสมาชิกรายเดือนเมษายนกับมกรารวมกัน";
    let answer5 = ask_question(app_state.clone(), auth_user.clone(), question5, None).await?;

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
    println!("\n========================================");
    println!("❓ Question 4: {question4}");
    println!("💡 Answer 4:\n---\n{answer4}\n---");
    println!("\n========================================");
    println!("❓ Question 5: {question5}");
    println!("💡 Answer 5:\n---\n{answer5}\n---");

    assert!(answer3.trim_matches('"').starts_with("สรุปเงื่อนไขได้ว่า"));
    assert!(answer4.trim_matches('"').contains("35,000"));
    assert!(answer5.trim_matches('"').contains("2,515,037"));

    Ok(())
    /* Expect
    ✅ Knowledge RAG Workflow Complete!
    ========================================
    ❓ Question 1: GPF Point แจกถึงวันไหน
    💡 Answer 1:
    ---
    "สมาชิกจะได้รับ GPF Point เมื่อสร้าง My GPF, My Twin 1, และ My Twin 2 เป็นครั้งแรก ภายในวันที่ 31 ธันวาคม 2567 หรือตามเงื่อนไขที่ กบข. กำหนด"
    ---

    ========================================
    ❓ Question 2: สร้าง My GPF ได้รับที่คะแนน
    💡 Answer 2:
    ---
    "สมาชิกจะได้รับ GPF Point 50 คะแนนต่อคนสำหรับการสร้าง My GPF, My Twin 1, และ My Twin 2 ครั้งแรก ภายในวันที่ 31 ธันวาคม 2567 หรือตามเงื่อนไขที่ กบข. กำหนด โดยจะได้รับ 10 คะแนนจากการสร้าง My GPF, 20 คะแนนจากการสร้าง My Twin 1, และ 20 คะแนนจากการสร้าง My Twin 2"
    ---

    ========================================
    ❓ Question 3: สร้าง My GPF ได้รับที่คะแนน
    💡 Answer 3:
    ---
    "สรุปเงื่อนไขได้ว่า สมาชิกจะได้รับ GPF Point ทั้งหมด 50 คะแนนต่อคน สำหรับการสร้าง My GPF, My Twin 1 และ My Twin 2 ครั้งแรก โดยจะได้รับ 10 คะแนนสำหรับการสร้าง My GPF, 20 คะแนนสำหรับการสร้าง My Twin 1 และ 20 คะแนนสำหรับการสร้าง My Twin 2 ทั้งนี้ ต้องดำเนินการภายในวันที่ 31 ธันวาคม 2567 หรือตามเงื่อนไขที่ กบข. กำหนด."
    ---

    ========================================
    ❓ Question 4: มีเงิน 2 หมื่นออมต่อได้มั้ย
    💡 Answer 4:
    ---
    "ไม่สามารถออมต่อได้ เพราะยอดเงินในบัญชีต้องไม่ต่ำกว่า 35,000 บาท ณ วันที่ยื่นคำขอ\n\nหลักเกณฑ์การออมต่อระบุว่า ยอดเงินในบัญชีต้องไม่ต่ำกว่า 35,000 บาท ณ วันที่ยื่นคำขอ โดยมีเอกสารหลักฐานถูกต้องครบถ้วน"
    ---
    */
}
