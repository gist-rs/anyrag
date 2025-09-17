use crate::ingest::{run_github_ingestion, storage::StorageManager, types::IngestionTask};
use anyhow::Result;
use anyrag::{constants, ingest::Ingestor};
use anyrag_markdown::{
    EmbeddingConfig as MarkdownEmbeddingConfig, MarkdownIngestor, MarkdownSource,
};
use clap::Parser;
use serde_json;
use std::fs;
use tracing::info;

#[derive(Parser, Debug)]
pub struct GithubArgs {
    /// The URL of the public GitHub repository to ingest
    #[arg(required = true)]
    pub url: String,
    /// An optional git version (tag, branch, commit hash) to ingest. Defaults to the latest release tag.
    #[arg(long)]
    pub version: Option<String>,
    /// Disables the automatic processing of the generated markdown file into chunks.
    #[arg(long)]
    pub no_process: bool,
    /// The API URL for the embedding model (optional). If provided, embeddings will be generated for chunks.
    #[arg(long, env = "EMBEDDINGS_API_URL")]
    pub embedding_api_url: Option<String>,
    /// The name of the embedding model to use (required if embedding-api-url is set).
    #[arg(long, env = "EMBEDDINGS_MODEL", requires = "embedding_api_url")]
    pub embedding_model: Option<String>,
}

pub async fn handle_dump_github(args: &GithubArgs) -> Result<()> {
    info!(
        "Starting GitHub ingestion for URL: {} with version: {:?}",
        args.url, args.version
    );
    println!("📥 Starting ingestion for '{}'...", args.url);

    let task = IngestionTask {
        url: args.url.clone(),
        version: args.version.clone(),
        embedding_api_url: args.embedding_api_url.clone(),
        embedding_model: args.embedding_model.clone(),
        embedding_api_key: std::env::var("AI_API_KEY").ok(),
    };

    let storage_manager = StorageManager::new(Some(constants::GITHUB_DB_DIR)).await?;
    let (ingested_count, ingested_version) = run_github_ingestion(&storage_manager, task).await?;
    println!(
        "✅ Successfully ingested {} unique examples from '{}' (version: {}).",
        ingested_count, args.url, ingested_version
    );

    if ingested_count == 0 {
        println!("No new examples were found to generate a context file.");
        return Ok(());
    }

    // Now, generate the markdown file from the ingested data.
    println!("📝 Generating consolidated context file...");
    let repo_name = StorageManager::url_to_repo_name(&args.url);

    let version_to_fetch = ingested_version;

    let examples = storage_manager
        .get_examples(&repo_name, &version_to_fetch)
        .await?;

    if examples.is_empty() {
        println!(
            "Could not find any examples in the database for version '{version_to_fetch}' to generate context file."
        );
        return Ok(());
    }

    // Sort examples by handle for a deterministic output.
    let mut sorted_examples = examples;
    sorted_examples.sort_by(|a, b| a.example_handle.cmp(&b.example_handle));

    // Add a header to the markdown content that specifies the repository and version.
    let mut markdown_content =
        format!("# Code Examples for {repo_name} (Version: {version_to_fetch})\n\n");

    let example_markdown = sorted_examples
        .iter()
        .map(|ex| {
            format!(
                "## `{}`\n**Source:** `{}` (`{}`)\n\n```rust\n{}\n```\n",
                ex.example_handle, ex.source_file, ex.source_type, ex.content
            )
        })
        .collect::<Vec<String>>()
        .join("---\n");

    markdown_content.push_str(&example_markdown);

    // Sanitize version string for the filename (e.g., replace `/` with `-`)
    let safe_version = version_to_fetch.replace('/', "-");
    let output_filename = format!("{repo_name}-{safe_version}-context.md");
    fs::write(&output_filename, markdown_content)?;
    println!("✅ Successfully generated context file: '{output_filename}'");

    // Automatically process the generated file into chunks unless disabled.
    if !args.no_process {
        println!("🚀 Automatically processing generated file into chunks...");
        let chunk_db_dir = constants::GITHUB_CHUNKS_DB_DIR;
        fs::create_dir_all(chunk_db_dir)?;
        let chunk_db_path = format!("{chunk_db_dir}/{repo_name}.db");

        let api_key = std::env::var("AI_API_KEY").ok();

        let embedding_config =
            if let (Some(url), Some(model)) = (&args.embedding_api_url, &args.embedding_model) {
                Some(MarkdownEmbeddingConfig {
                    api_url: url.clone(),
                    model: model.clone(),
                    api_key: api_key.clone(),
                })
            } else {
                None
            };

        let markdown_source = MarkdownSource {
            db_path: chunk_db_path.clone(),
            file_path: output_filename.clone(),
            separator: "---\n".to_string(),
            embedding_config,
        };

        let source_json = serde_json::to_string(&markdown_source)?;
        let ingestor = MarkdownIngestor;
        let result = ingestor.ingest(&source_json, None).await?;
        let count = result.documents_added;

        if count > 0 {
            println!(
                "✅ Successfully ingested {count} chunks from '{output_filename}' into '{chunk_db_path}'."
            );
        }
    }

    Ok(())
}
