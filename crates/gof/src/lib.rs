//! # `gof` Library Crate
//!
//! This crate contains the core logic for the `gof` CLI tool, which automates
//! the creation of a RAG knowledge base from a Rust project's dependencies.

use anyhow::{anyhow, Context, Result};

use anyrag::{
    providers::ai::{local::LocalAiProvider, AiProvider},
    SearchResult,
};
use clap::{Parser, Subcommand};
use crates_io_api::SyncClient;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf, sync::Arc};
use tracing::{error, info, warn};

// --- CLI Argument Structs ---

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Ingest code examples from all dependencies in a Cargo.toml
    Example(ExampleArgs),
    /// Search the ingested code examples using RAG
    Mcp(McpArgs),
}

#[derive(Parser, Debug)]
pub struct ExampleArgs {
    /// Path to the Cargo.toml file. Defaults to the current directory.
    #[arg(long, default_value = "./Cargo.toml")]
    path: PathBuf,
    /// The API URL for the embedding model (optional).
    #[arg(long, env = "EMBEDDINGS_API_URL")]
    embedding_api_url: Option<String>,
    /// The name of the embedding model to use (required if embedding-api-url is set).
    #[arg(long, env = "EMBEDDINGS_MODEL", requires = "embedding_api_url")]
    embedding_model: Option<String>,
}

#[derive(Parser, Debug)]
pub struct McpArgs {
    /// The search query.
    query: String,
    /// A list of repository names to search within (e.g., "tursodatabase-turso").
    /// If omitted, all ingested repositories will be searched.
    #[arg(long, value_delimiter = ',')]
    repos: Option<Vec<String>>,
}

// --- MCP Protocol Structs ---

#[derive(Serialize, Deserialize)]
pub struct McpSuccessResponse {
    pub results: Vec<McpSearchResult>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct McpSearchResult {
    // Note: repository and version are not available in the standard SearchResult.
    // This can be added later if needed. For now, we return what's available.
    pub source_file: String,
    pub handle: String,
    pub content: String,
    pub score: f64,
}

impl From<SearchResult> for McpSearchResult {
    fn from(value: SearchResult) -> Self {
        Self {
            source_file: value.link,
            handle: value.title,
            content: value.description,
            score: value.score,
        }
    }
}

#[derive(Serialize)]
struct McpErrorResponse {
    error: McpError,
}

#[derive(Serialize)]
struct McpError {
    code: String,
    message: String,
}

// --- Public Entrypoint ---

/// The main entry point for the `gof` library.
pub async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Example(args) => handle_example(args).await,
        Commands::Mcp(args) => handle_mcp(args).await,
    }
}

// --- Command Handlers ---

/// Handles the `gof example` command logic.
async fn handle_example(args: ExampleArgs) -> Result<()> {
    info!("Starting 'example' command with args: {:?}", args);
    println!(
        "🔎 Analyzing dependencies from '{}'...",
        args.path.display()
    );

    // 1. Parse Cargo.toml
    let dependencies = parse_dependencies(&args.path)?;
    if dependencies.is_empty() {
        println!(
            "🤷 No dependencies found in '{}'. Nothing to do.",
            args.path.display()
        );
        return Ok(());
    }
    println!(
        "Discovered {} dependencies. Resolving repository URLs...",
        dependencies.len()
    );

    // 2. Resolve Repository URLs
    let client = SyncClient::new(
        "anyrag-gof-cli (anyrag@example.com)",
        std::time::Duration::from_millis(1000),
    )
    .context("Failed to create crates.io API client")?;

    let mut repo_tasks = Vec::new();
    for (name, version) in dependencies {
        match client.get_crate(&name) {
            Ok(crate_info) => {
                if let Some(repo_url) = crate_info.crate_data.repository {
                    info!("Resolved '{}' to repository '{}'", name, repo_url);
                    repo_tasks.push((repo_url, version));
                } else {
                    warn!("Crate '{}' does not have a repository URL specified in its metadata. Skipping.", name);
                }
            }
            Err(e) => {
                error!(
                    "Failed to fetch metadata for crate '{}': {}. Skipping.",
                    name, e
                );
            }
        }
    }

    if repo_tasks.is_empty() {
        println!("\n🚫 Could not resolve any repository URLs. No examples will be ingested.");
        return Ok(());
    }

    println!(
        "\n🚀 Starting parallel ingestion for {} repositories...",
        repo_tasks.len()
    );

    // 3. Parallel Ingestion
    let storage_manager = anyrag_github::ingest::storage::StorageManager::new(None).await?;
    let mut handles = vec![];

    for (url, version) in repo_tasks {
        let storage_manager_clone = storage_manager.clone();
        let embedding_api_url_clone = args.embedding_api_url.clone();
        let embedding_model_clone = args.embedding_model.clone();
        let api_key = std::env::var("AI_API_KEY").ok();

        let handle = tokio::spawn(async move {
            let task = anyrag_github::ingest::types::IngestionTask {
                url: url.clone(),
                version: Some(version.clone()),
                embedding_api_url: embedding_api_url_clone,
                embedding_model: embedding_model_clone,
                embedding_api_key: api_key,
            };

            println!("  -> Starting ingestion for {url}@{version}");
            let result = anyrag_github::run_github_ingestion(&storage_manager_clone, task).await;

            match result {
                Ok((count, ingested_version)) => {
                    println!("  ✅ Finished {url}@{ingested_version}: Ingested {count} examples.");
                    Ok(())
                }
                Err(e) => {
                    eprintln!("  ❌ Error ingesting {url}@{version}: {e:?}");
                    Err(anyhow!("Ingestion failed for {url}"))
                }
            }
        });
        handles.push(handle);
    }

    let results = join_all(handles).await;
    let success_count = results
        .iter()
        .filter(|res| res.is_ok() && res.as_ref().unwrap().is_ok())
        .count();
    let fail_count = results.len() - success_count;

    println!("\n✨ Ingestion complete.");
    println!("   - {success_count} repositories succeeded.");
    if fail_count > 0 {
        println!("   - {fail_count} repositories failed.");
    }

    Ok(())
}

/// Parses a `Cargo.toml` file and extracts a list of (name, version) for dependencies.
pub fn parse_dependencies(path: &PathBuf) -> Result<Vec<(String, String)>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read Cargo.toml at '{}'", path.display()))?;
    let table: toml::Table = content
        .parse()
        .with_context(|| format!("Failed to parse TOML from '{}'", path.display()))?;

    let mut deps = Vec::new();

    if let Some(dependencies) = table.get("dependencies").and_then(|d| d.as_table()) {
        for (name, val) in dependencies {
            let version = if val.is_str() {
                val.as_str().unwrap().to_string()
            } else if let Some(table) = val.as_table() {
                table
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("*") // Default if version is not specified
                    .to_string()
            } else {
                "*".to_string()
            };
            deps.push((name.clone(), version));
        }
    }

    // Optionally, you could also parse [dev-dependencies], [build-dependencies], etc.

    Ok(deps)
}

/// Handles the `gof mcp` command logic.
async fn handle_mcp(args: McpArgs) -> Result<()> {
    info!("Starting 'mcp' command with args: {:?}", args);

    let result = run_mcp_search(args).await;

    match result {
        Ok(response_json) => {
            // On success, print JSON to stdout.
            println!("{response_json}");
            Ok(())
        }
        Err(e) => {
            // On failure, create a structured error, print to stderr, and return the error
            // to ensure a non-zero exit code.
            let error_response = McpErrorResponse {
                error: McpError {
                    code: "SearchError".to_string(),
                    message: e.to_string(),
                },
            };
            eprintln!("{}", serde_json::to_string(&error_response)?);
            Err(e)
        }
    }
}

/// The core logic for the MCP search, designed to return a JSON string on success
/// or an `anyhow::Error` on failure.
async fn run_mcp_search(args: McpArgs) -> Result<String> {
    // 1. Get repository list. For now, we require it to be specified.
    let repos_to_search = args.repos.ok_or_else(|| {
        anyhow!("The --repos flag must be provided with a list of repository names to search.")
    })?;

    if repos_to_search.is_empty() {
        return Err(anyhow!("The --repos list cannot be empty."));
    }

    // 2. Get embedding configuration from environment.
    let embedding_api_url = env::var("EMBEDDINGS_API_URL")
        .context("EMBEDDINGS_API_URL environment variable is not set")?;
    let embedding_model =
        env::var("EMBEDDINGS_MODEL").context("EMBEDDINGS_MODEL environment variable is not set")?;
    let embedding_api_key = env::var("AI_API_KEY").ok();

    // 3. Create dependencies (StorageManager, AiProvider).
    let storage_manager = anyrag_github::ingest::storage::StorageManager::new(None).await?;
    let local_ai_url = env::var("LOCAL_AI_API_URL")
        .or_else(|_| env::var("AI_API_URL"))
        .context("LOCAL_AI_API_URL or AI_API_URL must be set for local provider")?;
    let ai_api_key = env::var("AI_API_KEY").ok();
    let ai_model = env::var("AI_MODEL").ok();

    let ai_provider: Arc<dyn AiProvider> = Arc::new(
        LocalAiProvider::new(local_ai_url, ai_api_key, ai_model)
            .context("Failed to create LocalAiProvider")?,
    );

    // 4. Call the search function.
    info!(
        "Executing search for '{}' in repos: {:?}",
        args.query, repos_to_search
    );
    let search_results = anyrag_github::search_examples(
        &storage_manager,
        &args.query,
        &repos_to_search,
        ai_provider,
        &embedding_api_url,
        &embedding_model,
        embedding_api_key.as_deref(),
    )
    .await
    .context("The search operation failed")?;

    // 5. Format the successful response according to the MCP protocol.
    format_mcp_response(search_results)
}

/// Formats a vector of `SearchResult` into the MCP JSON string.
pub fn format_mcp_response(search_results: Vec<SearchResult>) -> Result<String> {
    let mcp_results: Vec<McpSearchResult> = search_results
        .into_iter()
        .map(McpSearchResult::from)
        .collect();
    let response = McpSuccessResponse {
        results: mcp_results,
    };

    serde_json::to_string_pretty(&response)
        .context("Failed to serialize successful response to JSON")
}
