use anyhow::Result;
use anyrag::github_ingest::{run_github_ingestion, storage::StorageManager, types::IngestionTask};
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
    };

    let ingested_count = run_github_ingestion(task).await?;
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
    let storage_manager = StorageManager::new("db/github_ingest").await?;
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

    let markdown_content = sorted_examples
        .iter()
        .map(|ex| {
            format!(
                "## `{}`\n**Source:** `{}` (`{}`)\n\n```rust\n{}\n```\n",
                ex.example_handle, ex.source_file, ex.source_type, ex.content
            )
        })
        .collect::<Vec<String>>()
        .join("---\n");

    let output_filename = format!("{repo_name}-context.md");
    fs::write(&output_filename, markdown_content)?;
    println!("✅ Successfully generated context file: '{output_filename}'");

    Ok(())
}
