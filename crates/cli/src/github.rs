use anyhow::Result;
use anyrag::{
    github_ingest::{run_github_ingestion, storage::StorageManager, types::IngestionTask},
    ingest::markdown::EmbeddingConfig,
};
use clap::Parser;
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
    };

    let storage_manager = StorageManager::new("db/github_ingest").await?;
    let ingested_count = run_github_ingestion(&storage_manager, task).await?;
    println!(
        "✅ Successfully ingested {} unique examples from '{}'.",
        ingested_count, args.url
    );

    if ingested_count == 0 {
        println!("No new examples were found to generate a context file.");
        return Ok(());
    }

    // Now, generate the markdown file from the ingested data.
    println!("📝 Generating consolidated context file...");
    let repo_name = StorageManager::url_to_repo_name(&args.url);

    // Determine which version to fetch. If a version was specified for ingestion, use that.
    // Otherwise, ask the storage manager for the latest version it has.
    let version_to_fetch = match &args.version {
        Some(v) => v.clone(),
        None => storage_manager
            .get_latest_version(&repo_name)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Could not determine latest version for '{repo_name}' after ingestion."
                )
            })?,
    };

    let examples = storage_manager
        .get_examples(&repo_name, &version_to_fetch)
        .await?;

    if examples.is_empty() {
        println!("Could not find any examples in the database for version '{version_to_fetch}' to generate context file.");
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
        let chunk_db_dir = "db/github_chunks";
        fs::create_dir_all(chunk_db_dir)?;
        let chunk_db_path = format!("{chunk_db_dir}/{repo_name}.db");

        let embedding_config = if let (Some(url), Some(model)) = (
            args.embedding_api_url.as_deref(),
            args.embedding_model.as_deref(),
        ) {
            Some(EmbeddingConfig {
                api_url: url,
                model,
            })
        } else {
            None
        };

        let count = anyrag::ingest::markdown::ingest_markdown_file(
            &chunk_db_path,
            &output_filename,
            "---\n", // The separator used for joining examples
            embedding_config,
        )
        .await?;

        if count > 0 {
            println!(
                "✅ Successfully ingested {count} chunks from '{output_filename}' into '{chunk_db_path}'."
            );
        }
    }

    Ok(())
}
