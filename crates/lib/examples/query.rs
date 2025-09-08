//! Example: Programmatic Text-to-SQL for BigQuery.
//!
//! This example demonstrates how to use the `anyrag` library's core Text-to-SQL
//! functionality to query a Google BigQuery table programmatically. It acts as a
//! command-line interface, accepting a flexible JSON string that maps directly
//! to the `ExecutePromptOptions`.
//!
//! # Workflow:
//! 1.  **Setup**: Initializes the AI and BigQuery providers from environment variables.
//! 2.  **Argument Parsing**: Reads a JSON string from the command line.
//! 3.  **Execution**: Calls `execute_prompt_from_value` to perform the full
//!     Text-to-SQL pipeline: generate SQL, execute it, and format the result.
//! 4.  **Output**: Prints the final answer, the generated SQL, and the raw
//!     database result.
//!
//! # Prerequisites
//!
//! - A valid `.env` file in the `crates/lib` directory with credentials for the
//!   AI provider (`AI_API_URL`, `AI_API_KEY`) and BigQuery (`BIGQUERY_PROJECT_ID`).
//! - Local Google Cloud authentication. Run `gcloud auth application-default login`.
//!
//! # Usage
//!
//! From the workspace root (`anyrag/`):
//! ```sh
//! cargo run -p anyrag --example query --features="bigquery" -- '{"prompt": "How many distinct corpuses are there?", "table_name": "bigquery-public-data.samples.shakespeare", "instruction": "Answer with only the number."}'
//! ```
//! Note the `--` which separates Cargo arguments from the example's arguments.

use anyrag::{
    providers::ai::{gemini::GeminiProvider, local::LocalAiProvider},
    PromptClientBuilder,
};
use dotenvy::dotenv;
use serde_json::Value;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging and load .env file
    tracing_subscriber::fmt::init();
    dotenv().ok();

    // --- Command-line argument parsing ---
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} '<json_options>'", args[0]);
        eprintln!();
        eprintln!("Example: {} '{{\"prompt\": \"Count the corpus in the shakespeare dataset\", \"table_name\": \"bigquery-public-data.samples.shakespeare\"}}'", args[0]);
        return Ok(());
    }

    // The entire options struct is passed as a single JSON string
    let options_json = &args[1];
    let options_value: Value = serde_json::from_str(options_json)
        .expect("Failed to parse the first argument as valid JSON.");

    // --- Configuration from environment variables ---
    let ai_provider_name = env::var("AI_PROVIDER").unwrap_or_else(|_| "gemini".to_string());
    // Use LOCAL_AI_API_URL for local providers, AI_API_KEY for Gemini.
    let local_api_url =
        env::var("LOCAL_AI_API_URL").expect("LOCAL_AI_API_URL must be set for this example");
    let api_key = env::var("AI_API_KEY").ok();
    let ai_model = env::var("AI_MODEL").ok();
    let project_id =
        env::var("BIGQUERY_PROJECT_ID").expect("BIGQUERY_PROJECT_ID environment variable not set");

    // --- Build AI Provider ---
    let ai_provider = match ai_provider_name.as_str() {
        "gemini" => {
            let key = api_key.expect("AI_API_KEY is required for gemini provider");
            // The Gemini URL is now hardcoded in the provider, so we don't pass it here.
            Box::new(GeminiProvider::new("".to_string(), key)?)
                as Box<dyn anyrag::providers::ai::AiProvider>
        }
        "local" => Box::new(LocalAiProvider::new(local_api_url, api_key, ai_model)?)
            as Box<dyn anyrag::providers::ai::AiProvider>,
        _ => return Err(format!("Unsupported AI provider: {ai_provider_name}").into()),
    };

    // --- Build Prompt Client ---
    let client = PromptClientBuilder::new()
        .ai_provider(ai_provider)
        .bigquery_storage(project_id)
        .await?
        .build()?;

    // --- Execute Prompt ---
    // The `execute_prompt_from_value` method allows for maximum flexibility,
    // as any field in `ExecutePromptOptions` can be passed in the JSON.
    match client.execute_prompt_from_value(options_value).await {
        Ok(prompt_result) => {
            println!("--- Final Result ---");
            println!("{}", prompt_result.text);

            if let Some(sql) = prompt_result.generated_sql {
                println!("\n--- Generated SQL ---");
                println!("{sql}");
            }

            if let Some(db_result) = prompt_result.database_result {
                println!("\n--- Raw Database Result ---");
                println!("{db_result}");
            }
        }
        Err(e) => eprintln!("Error: {e}"),
    }

    Ok(())
}
