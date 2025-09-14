use anyhow::Result;
use anyrag::ingest::ingest_markdown_file;
use clap::{Parser, Subcommand};
use std::path::Path;
use tracing::info;

#[derive(Parser, Debug)]
pub struct ProcessArgs {
    #[command(subcommand)]
    command: ProcessCommands,
}

#[derive(Subcommand, Debug)]
enum ProcessCommands {
    /// Process a local file for ingestion
    File(FileArgs),
}

#[derive(Parser, Debug)]
struct FileArgs {
    /// The path to the local file to process
    #[arg(required = true)]
    path: String,
    /// The path to the database file to use for storage
    #[arg(long, default_value = "db/anyrag_processed.db")]
    db_path: String,
    /// The separator string used to split the file content into chunks
    #[arg(long, default_value = "\n---\n")]
    separator: String,
}

pub async fn handle_process(args: &ProcessArgs) -> Result<()> {
    match &args.command {
        ProcessCommands::File(file_args) => handle_process_file(file_args).await,
    }
}

async fn handle_process_file(args: &FileArgs) -> Result<()> {
    info!("Processing file: {}", args.path);
    println!("📄 Processing file: '{}'...", args.path);

    // Ensure the db directory exists before trying to create the database.
    if let Some(parent) = Path::new(&args.db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let count = ingest_markdown_file(&args.db_path, &args.path, &args.separator).await?;

    println!(
        "✅ Successfully ingested {} chunks from '{}' into '{}'.",
        count, args.path, args.db_path
    );

    Ok(())
}
