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

use anyhow::Result;
use anyrag::ingest::Ingestor;
use anyrag_notion::NotionIngestor;
use dotenvy::dotenv;
use serde_json::json;
use std::env;
use turso::params;

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

    // --- 3. Verification ---
    if result.documents_added > 0 {
        if let Some(metadata_str) = &result.metadata {
            println!("\n--- Verifying ingested data ---");
            let metadata: serde_json::Value = serde_json::from_str(metadata_str)?;
            let db_file = metadata["db_file"].as_str().unwrap_or("N/A");
            let table_name = &result.document_ids[0];

            let db = turso::Builder::new_local(db_file).build().await?;
            let conn = db.connect()?;
            let mut stmt = conn
                .prepare(&format!("SELECT COUNT(*) FROM `{table_name}`"))
                .await?;
            let count: i64 = stmt.query_row(params![]).await?.get(0)?;

            println!("Verification successful: Found {count} rows in table '{table_name}'.");
            assert_eq!(count as usize, result.documents_added);
        }
    } else {
        println!("\n--- Verification Skipped (no documents were added) ---");
    }

    Ok(())
}
