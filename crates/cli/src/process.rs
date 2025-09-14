use anyhow::Result;
use anyrag::ingest::markdown::{ingest_markdown_file, EmbeddingConfig};
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
    /// The API URL for the embedding model (optional). If provided, embeddings will be generated.
    #[arg(long, env = "EMBEDDINGS_API_URL")]
    embedding_api_url: Option<String>,
    /// The name of the embedding model to use (required if embedding-api-url is set).
    #[arg(long, env = "EMBEDDINGS_MODEL", requires = "embedding_api_url")]
    embedding_model: Option<String>,
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

    let embedding_config = args.embedding_api_url.as_deref().and_then(|url| {
        args.embedding_model
            .as_deref()
            .map(|model| EmbeddingConfig {
                api_url: url,
                model,
            })
    });

    let count =
        ingest_markdown_file(&args.db_path, &args.path, &args.separator, embedding_config).await?;

    println!(
        "✅ Successfully ingested {} chunks from '{}' into '{}'.",
        count, args.path, args.db_path
    );

    Ok(())
}
