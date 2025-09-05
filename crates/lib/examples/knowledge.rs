//! Example: Programmatic End-to-end Knowledge Base RAG workflow.
//!
//! This example demonstrates the full "virtuous cycle" workflow using the library's
//! core functions, without running the web server. It shows how to use the `anyrag`
//! crate as a library to build a RAG pipeline.
//!
//! # Workflow:
//! 1.  **Setup**: Initializes AI and Storage providers.
//! 2.  **Ingestion**: Ingests content from a real-world URL into the knowledge base.
//! 3.  **Embedding**: Generates vector embeddings for the newly created documents.
//! 4.  **RAG**: Asks a question against the new knowledge, performing the full
//!     multi-stage hybrid search and synthesizing a final answer.
//!
//! # Prerequisites
//!
//! - A valid `.env` file in the `crates/lib` directory with credentials
//!   for a running AI and embedding provider.
//! - An internet connection to fetch the URL.
//!
//! # Usage
//!
//! From the workspace root (`anyrag/`):
//! `RUST_LOG=info cargo run -p anyrag --example knowledge --features="core-access"`

use anyhow::Result;
use anyrag::{
    ingest::{knowledge::IngestionPrompts, run_ingestion_pipeline, KnowledgeError},
    prompts::knowledge::{
        AUGMENTATION_SYSTEM_PROMPT, KNOWLEDGE_EXTRACTION_SYSTEM_PROMPT,
        KNOWLEDGE_EXTRACTION_USER_PROMPT, METADATA_EXTRACTION_SYSTEM_PROMPT,
        QUERY_ANALYSIS_SYSTEM_PROMPT, QUERY_ANALYSIS_USER_PROMPT,
    },
    providers::{
        ai::{gemini::GeminiProvider, generate_embedding, local::LocalAiProvider},
        db::sqlite::SqliteProvider,
    },
    search::{hybrid_search, HybridSearchPrompts},
    types::{ContentType, ExecutePromptOptions},
    PromptClientBuilder,
};
use core_access::get_or_create_user;
use dotenvy::dotenv;
use std::{env, fs};
use tokio::time::{sleep, Duration};
use tracing::info;
use tracing_subscriber::EnvFilter;
use turso::params;

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- 1. Setup ---
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    dotenv().ok();
    info!("Environment variables loaded.");

    // Use a dedicated, temporary DB for this example run.
    let db_path = "db/anyrag_lib_example.db";
    cleanup_db(db_path).await?;

    // --- Configuration from environment variables ---
    let ai_provider_name = env::var("AI_PROVIDER").unwrap_or_else(|_| "gemini".to_string());
    let ai_api_url = env::var("AI_API_URL").expect("AI_API_URL environment variable not set");
    let ai_api_key = env::var("AI_API_KEY").ok();
    let ai_model = env::var("AI_MODEL").ok();
    let embeddings_api_url =
        env::var("EMBEDDINGS_API_URL").expect("EMBEDDINGS_API_URL environment variable not set");
    let embeddings_model =
        env::var("EMBEDDINGS_MODEL").expect("EMBEDDINGS_MODEL environment variable not set");

    // --- Build AI Provider ---
    let ai_provider = match ai_provider_name.as_str() {
        "gemini" => {
            let key = ai_api_key.expect("AI_API_KEY is required for gemini provider");
            Box::new(GeminiProvider::new(ai_api_url, key)?)
                as Box<dyn anyrag::providers::ai::AiProvider>
        }
        "local" => Box::new(LocalAiProvider::new(ai_api_url, ai_api_key, ai_model)?)
            as Box<dyn anyrag::providers::ai::AiProvider>,
        _ => return Err(format!("Unsupported AI provider: {ai_provider_name}").into()),
    };

    // --- Build Storage Provider ---
    let sqlite_provider = SqliteProvider::new(db_path).await?;
    sqlite_provider.initialize_schema().await?;
    info!("SQLite provider initialized and schema is ready.");

    sleep(Duration::from_millis(100)).await;

    // --- 2. Ingest Knowledge (with Ownership) ---
    info!("--- Starting Knowledge Ingestion ---");
    let ingest_url = "https://www.true.th/betterliv/support/true-app-mega-campaign";
    // Create a user to own the ingested content.
    let user = get_or_create_user(&sqlite_provider.db, "default-user@example.com").await?;
    info!("Content will be ingested for owner_id: {}", user.id);

    let prompts = IngestionPrompts {
        extraction_system_prompt: KNOWLEDGE_EXTRACTION_SYSTEM_PROMPT,
        extraction_user_prompt_template: KNOWLEDGE_EXTRACTION_USER_PROMPT,
        augmentation_system_prompt: AUGMENTATION_SYSTEM_PROMPT,
        metadata_extraction_system_prompt: METADATA_EXTRACTION_SYSTEM_PROMPT,
    };
    match run_ingestion_pipeline(
        &sqlite_provider.db,
        ai_provider.as_ref(),
        ingest_url,
        Some(&user.id),
        prompts,
    )
    .await
    {
        Ok(count) => {
            info!("Ingestion successful. Stored {} new FAQs.", count);
            if count == 0 {
                info!("Content may be unchanged from a previous run. Continuing...");
            }
        }
        Err(KnowledgeError::ContentUnchanged(_)) => {
            info!("Content is unchanged. Skipping ingestion.");
        }
        Err(e) => {
            return Err(format!(
                "Knowledge ingestion failed: {e:?}. Please ensure your AI provider is running."
            )
            .into());
        }
    }

    // --- 3. Embed New Documents ---
    info!("--- Starting Embedding for New Documents ---");
    let conn = sqlite_provider.db.connect()?;
    let mut stmt = conn
        .prepare(
            "SELECT d.id, d.title, d.content FROM documents d
             LEFT JOIN document_embeddings de ON d.id = de.document_id
             WHERE de.id IS NULL",
        )
        .await?;
    let mut rows = stmt.query(()).await?;

    let mut embed_count = 0;
    while let Some(row) = rows.next().await? {
        let doc_id: String = row.get(0)?;
        let title: String = row.get(1)?;
        let content: String = row.get(2)?;
        let text_to_embed = format!("{title}. {content}");

        let vector =
            generate_embedding(&embeddings_api_url, &embeddings_model, &text_to_embed).await?;
        let vector_bytes: &[u8] =
            unsafe { std::slice::from_raw_parts(vector.as_ptr() as *const u8, vector.len() * 4) };

        conn.execute(
            "INSERT INTO document_embeddings (document_id, model_name, embedding) VALUES (?, ?, ?)",
            params![doc_id.clone(), embeddings_model.clone(), vector_bytes],
        )
        .await?;
        embed_count += 1;
        info!("Successfully embedded document ID: {}", doc_id);
    }
    info!(
        "Embedding complete. Processed {} new documents.",
        embed_count
    );

    // --- 4. Ask a Question using RAG ---
    info!("--- Starting RAG Search & Synthesis ---");
    let question = "ทำยังไงถึงจะได้เทสล่า";
    let instruction = "สรุปเงื่อนไขการรับสิทธิ์ลุ้นเทสล่า";

    let query_vector = generate_embedding(&embeddings_api_url, &embeddings_model, question).await?;

    let search_results = hybrid_search(
        &sqlite_provider,
        ai_provider.as_ref(),
        query_vector,
        question,
        Some(&user.id), // owner_id
        5,              // limit
        HybridSearchPrompts {
            analysis_system_prompt: QUERY_ANALYSIS_SYSTEM_PROMPT,
            analysis_user_prompt_template: QUERY_ANALYSIS_USER_PROMPT,
        },
    )
    .await?;

    let context = search_results
        .iter()
        .map(|r| r.description.clone()) // The 'description' holds the full content for documents
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    if context.is_empty() {
        println!("Could not find any relevant information.");
        return Ok(());
    }

    info!("Retrieved context for synthesis:\n---\n{}---", context);

    let client = PromptClientBuilder::new()
        .ai_provider(ai_provider)
        // Storage provider is not used for RAG synthesis, but is required to build the client.
        .storage_provider(Box::new(sqlite_provider))
        .build()?;

    let options = ExecutePromptOptions {
        prompt: question.to_string(),
        content_type: Some(ContentType::Knowledge),
        context: Some(context),
        instruction: Some(instruction.to_string()),
        ..Default::default()
    };

    let final_result = client.execute_prompt_with_options(options).await?;

    // --- 5. Print Final Results ---
    println!("\n\n✅ Programmatic RAG Workflow Complete!");
    println!("========================================");
    println!("❓ Question: {question}");
    println!("💡 Answer:\n---\n{}\n---", final_result.text);

    Ok(())
}
