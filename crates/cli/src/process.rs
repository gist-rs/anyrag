use anyhow::Result;
use anyrag::ingest::Ingestor;
use anyrag_markdown::{EmbeddingConfig, MarkdownIngestor, MarkdownSource};
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
    #[arg(long, default_value = anyrag::constants::DEFAULT_DB_FILE)]
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

    let embedding_api_key = std::env::var("AI_API_KEY").ok();
    let embedding_config =
        if let (Some(url), Some(model)) = (&args.embedding_api_url, &args.embedding_model) {
            Some(EmbeddingConfig {
                api_url: url.clone(),
                model: model.clone(),
                api_key: embedding_api_key,
            })
        } else {
            None
        };

    let markdown_source = MarkdownSource {
        db_path: args.db_path.clone(),
        file_path: args.path.clone(),
        separator: args.separator.clone(),
        embedding_config,
    };

    let ingestor = MarkdownIngestor;
    let source_json = serde_json::to_string(&markdown_source)?;
    let result = ingestor.ingest(&source_json, None).await.map_err(|e| {
        if e.to_string().contains("Embedding generation failed") {
            anyhow::anyhow!("Embedding generation failed")
        } else {
            anyhow::anyhow!(e)
        }
    })?;
    let count = result.documents_added;

    println!(
        "✅ Successfully ingested {} chunks from '{}' into '{}'.",
        count, args.path, args.db_path
    );

    Ok(())
}
