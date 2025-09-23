//! # Notion Ingestion Example
//!
//! This example demonstrates how to use the `NotionIngestor` to ingest data
//! from a real Notion database into an in-memory Turso database.
//!
//! ## Prerequisites
//!
//! 1.  Create a `.env` file in the root of the `anyrag` workspace.
//! 2.  Add the following environment variables to your `.env` file:
//!
//!     ```env
//!     NOTION_TOKEN="your_notion_integration_token"
//!     NOTION_VERSION="2022-06-28"
//!     NOTION_TEST_DB_ID="the_id_of_your_notion_database"
//!     ```
//!
//! ## How to Run
//!
//! From the `anyrag` workspace root, execute the following command:
//!
//! ```sh
//! cargo run -p anyrag-notion --example ingest_notion
//! ```

use anyhow::{anyhow, Result};
use anyrag::{
    ingest::Ingestor,
    providers::{
        ai::{gemini::GeminiProvider, local::LocalAiProvider, AiProvider},
        db::sqlite::SqliteProvider,
    },
    ExecutePromptOptions, PromptClientBuilder,
};
use anyrag_notion::NotionIngestor;
use chrono::Utc;
use dotenvy::dotenv;
use serde_json::json;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize a simple logger to view the output from the ingestor.
    tracing_subscriber::fmt::init();

    // Load environment variables from the .env file in the workspace root.
    dotenv().ok();

    // --- 1. Setup ---
    println!("--- Setting up the environment ---");
    let db_id = env::var("NOTION_TEST_DB_ID")
        .expect("NOTION_TEST_DB_ID must be set in your .env file. See example instructions.");
    let owner_id = "notion-example-user";

    println!("Target Notion Database ID: {db_id}");

    // --- 2. Ingestion ---
    println!("\n--- Starting Notion ingestion ---");
    let ingestor = NotionIngestor::new();
    let source = json!({ "database_id": db_id }).to_string();

    let result = ingestor.ingest(&source, Some(owner_id)).await?;

    println!("\n--- Ingestion Complete ---");
    println!("Source Database ID: {}", result.source);
    if let Some(metadata_str) = &result.metadata {
        let metadata: serde_json::Value = serde_json::from_str(metadata_str)?;
        let data_source_id = metadata["data_source_id"].as_str().unwrap_or("N/A");
        let table_name = metadata["table_name"].as_str().unwrap_or("N/A");
        let db_file = metadata["db_file"].as_str().unwrap_or("N/A");

        println!("Discovered Data Source ID: {data_source_id}");
        println!("Generated Table Name: {table_name}");
        println!("Data saved to database file: {db_file}");
        println!(
            "-> DB file name derived from: notion_{{md5({}::{})}}.db",
            result.source, data_source_id
        );
    } else {
        println!("Generated Table Name(s): {:?}", result.document_ids);
    }
    println!("Documents (rows) added: {}", result.documents_added);

    // --- 3. NL-to-SQL Search ---
    if result.documents_added > 0 {
        if let Some(metadata_str) = &result.metadata {
            println!("\n--- Verifying with NL-to-SQL Search ---");

            // --- AI Provider Setup ---
            let ai_provider_name = env::var("AI_PROVIDER").unwrap_or_else(|_| "local".to_string());
            let local_ai_api_url =
                env::var("LOCAL_AI_API_URL").expect("LOCAL_AI_API_URL must be set");
            let ai_api_key = env::var("AI_API_KEY").ok();
            let ai_model = env::var("AI_MODEL").ok();

            let ai_provider: Box<dyn AiProvider> = match ai_provider_name.as_str() {
                "gemini" => {
                    let key = ai_api_key.expect("AI_API_KEY is required for gemini provider");
                    let gemini_url = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-lite:generateContent";
                    Box::new(GeminiProvider::new(gemini_url.to_string(), key)?)
                }
                "local" => Box::new(LocalAiProvider::new(
                    local_ai_api_url,
                    ai_api_key,
                    ai_model,
                )?),
                _ => return Err(anyhow!("Unsupported AI provider: {}", ai_provider_name)),
            };

            // --- Storage Provider Setup ---
            let metadata: serde_json::Value = serde_json::from_str(metadata_str)?;
            let db_file = metadata["db_file"].as_str().unwrap_or("N/A");
            let table_name = &result.document_ids[0];
            let sqlite_provider = SqliteProvider::new(db_file).await?;

            // --- Prompt Execution ---
            let question = "Who is available today?";
            let today = Utc::now().to_rfc3339();
            println!("# CONTEXT\n- # TODAY: {}", today);
            println!("# QUERY\n- {}", question);

            let client = PromptClientBuilder::new()
                .ai_provider(ai_provider)
                .storage_provider(Box::new(sqlite_provider))
                .build()?;

            let options = ExecutePromptOptions {
                prompt: question.to_string(),
                table_name: Some(table_name.clone()),
                // Provide an instruction for a more direct answer
                instruction: Some(
                    "List the names of the available people. If no one is available, say so."
                        .to_string(),
                ),
                ..Default::default()
            };

            let final_result = client.execute_prompt_with_options(options).await?;

            println!("\n# RESPONSE");
            println!("- AI Generated Answer:\n{}", final_result.text);
            if let Some(sql) = final_result.generated_sql {
                println!("\n- Generated SQL:\n{}", sql);
            }
        }
    } else {
        println!("\n--- Search Skipped (no documents were added) ---");
    }

    Ok(())
}
