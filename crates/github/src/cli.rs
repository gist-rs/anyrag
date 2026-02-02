use crate::ingest::{
    crawler::Crawler, extractor::Extractor, run_github_ingestion, storage::StorageManager,
    types::IngestionTask,
};
use anyhow::Result;
use anyrag::{constants, ingest::Ingestor};
use anyrag_markdown::{
    EmbeddingConfig as MarkdownEmbeddingConfig, MarkdownIngestor, MarkdownSource,
};
use clap::{Parser, ValueEnum};
use serde_json;
use std::fs;
use tracing::info;

#[derive(ValueEnum, Clone, Debug, Default)]
#[clap(rename_all = "kebab_case")]
pub enum DumpType {
    #[default]
    Examples,
    Src,
    Tests,
}

#[derive(Parser, Debug)]
pub struct GithubArgs {
    /// The URL of the public GitHub repository to ingest
    #[arg(long, required = true)]
    pub url: String,
    /// An optional git version (tag, branch, commit hash) to ingest. Defaults to the latest release tag.
    #[arg(long)]
    pub version: Option<String>,
    /// The type of content to dump (examples or all source files).
    #[arg(long, value_enum, default_value_t = DumpType::Examples)]
    pub dump_type: DumpType,
    /// A comma-separated list of glob patterns to ignore (e.g., "*.lock,LICENSE").
    #[arg(long, use_value_delimiter = true)]
    pub ignore: Option<Vec<String>>,
    /// Disables the automatic processing of the generated markdown file into chunks.
    #[arg(long)]
    pub no_process: bool,
    /// The API URL for the embedding model (optional). If provided, embeddings will be generated for chunks.
    #[arg(long, env = "EMBEDDINGS_API_URL")]
    pub embedding_api_url: Option<String>,
    /// The name of the embedding model to use (required if embedding-api-url is set).
    #[arg(long, env = "EMBEDDINGS_MODEL", requires = "embedding_api_url")]
    pub embedding_model: Option<String>,
    /// Extract and embed files referenced by `include_bytes!` macros.
    #[arg(long)]
    pub extract_included_files: bool,
}

pub async fn handle_dump_github(args: &GithubArgs) -> Result<()> {
    match args.dump_type {
        DumpType::Examples => handle_examples_dump(args).await,
        DumpType::Src => handle_src_dump(args).await,
        DumpType::Tests => handle_tests_dump(args).await,
    }
}

async fn handle_examples_dump(args: &GithubArgs) -> Result<()> {
    info!(
        "Starting GitHub EXAMPLES ingestion for URL: {} with version: {:?}",
        args.url, args.version
    );
    println!("📥 Starting examples ingestion for '{}'...", args.url);

    let task = IngestionTask {
        url: args.url.clone(),
        version: args.version.clone(),
        embedding_api_url: args.embedding_api_url.clone(),
        embedding_model: args.embedding_model.clone(),
        embedding_api_key: std::env::var("AI_API_KEY").ok(),
        extract_included_files: args.extract_included_files,
        dump_type: crate::ingest::types::DumpType::Examples,
    };

    let storage_manager = StorageManager::new(Some(constants::GITHUB_DB_DIR)).await?;
    let (ingested_count, ingested_version) = run_github_ingestion(&storage_manager, task).await?;
    println!(
        "✅ Successfully ingested {} unique examples from '{}' (version: {}).",
        ingested_count, args.url, ingested_version
    );

    if ingested_count == 0 {
        println!("No new examples were found to generate a markdown file.");
        return Ok(());
    }

    println!("📝 Generating consolidated examples file...");
    let repo_name = StorageManager::url_to_repo_name(&args.url);

    let examples = storage_manager
        .get_examples(&repo_name, &ingested_version)
        .await?;

    if examples.is_empty() {
        println!("Could not find any examples in the database to generate the file.");
        return Ok(());
    }

    let mut sorted_examples = examples;
    sorted_examples.sort_by(|a, b| a.example_handle.cmp(&b.example_handle));

    let mut markdown_content =
        format!("# Code Examples for {repo_name} (Version: {ingested_version})\n\n");

    let example_markdown = sorted_examples
        .iter()
        .map(|ex| {
            let path = std::path::Path::new(&ex.source_file);
            let language = path
                .extension()
                .and_then(|s| s.to_str())
                .map(|ext| match ext {
                    "rs" => "rust",
                    "toml" => "toml",
                    "json" => "json",
                    "yaml" | "yml" => "yaml",
                    "md" => "markdown",
                    _ => "text",
                })
                .unwrap_or("text");

            format!(
                "## `{}`\n**Source:** `{}` (`{}`)\n\n```{}\n{}\n```\n",
                ex.example_handle, ex.source_file, ex.source_type, language, ex.content
            )
        })
        .collect::<Vec<String>>()
        .join("---\n");

    markdown_content.push_str(&example_markdown);

    let safe_version = ingested_version.replace('/', "-");
    let output_filename = format!("{repo_name}-{safe_version}-examples.md");
    fs::write(&output_filename, markdown_content)?;
    println!("✅ Successfully generated examples file: '{output_filename}'");

    if !args.no_process {
        let chunk_db_dir = format!("{}/examples", constants::GITHUB_CHUNKS_DB_DIR);
        process_markdown_file(args, &output_filename, &repo_name, &chunk_db_dir).await?;
    }

    Ok(())
}

async fn handle_tests_dump(args: &GithubArgs) -> Result<()> {
    info!(
        "Starting GitHub TESTS ingestion for URL: {} with version: {:?}",
        args.url, args.version
    );
    println!("📥 Starting tests ingestion for '{}'...", args.url);

    let task = IngestionTask {
        url: args.url.clone(),
        version: args.version.clone(),
        embedding_api_url: args.embedding_api_url.clone(),
        embedding_model: args.embedding_model.clone(),
        embedding_api_key: std::env::var("AI_API_KEY").ok(),
        extract_included_files: false, // Tests typically don't use include_bytes!
        dump_type: crate::ingest::types::DumpType::Tests,
    };

    let storage_manager = StorageManager::new(Some(constants::GITHUB_DB_DIR)).await?;
    let (ingested_count, ingested_version) = run_github_ingestion(&storage_manager, task).await?;
    println!(
        "✅ Successfully ingested {} unique tests from '{}' (version: {}).",
        ingested_count, args.url, ingested_version
    );

    if ingested_count == 0 {
        println!("No new tests were found to generate a markdown file.");
        return Ok(());
    }

    println!("📝 Generating consolidated tests file...");
    let repo_name = StorageManager::url_to_repo_name(&args.url);

    let tests = storage_manager
        .get_examples(&repo_name, &ingested_version)
        .await?;

    if tests.is_empty() {
        println!("Could not find any tests in the database to generate the file.");
        return Ok(());
    }

    let mut sorted_tests = tests;
    sorted_tests.sort_by(|a, b| a.example_handle.cmp(&b.example_handle));

    let mut markdown_content =
        format!("# Test Cases for {repo_name} (Version: {ingested_version})\n\n");

    let test_markdown = sorted_tests
        .iter()
        .map(|ex| {
            let _path = std::path::Path::new(&ex.source_file);
            let language = "rust";

            format!(
                "## `{}`\n**Source:** `{}` (`{}`)\n\n```{}\n{}\n```\n",
                ex.example_handle, ex.source_file, ex.source_type, language, ex.content
            )
        })
        .collect::<Vec<String>>()
        .join("---\n");

    markdown_content.push_str(&test_markdown);

    let safe_version = ingested_version.replace('/', "-");
    let output_filename = format!("{repo_name}-{safe_version}-tests.md");
    fs::write(&output_filename, markdown_content)?;
    println!("✅ Successfully generated tests file: '{output_filename}'");

    if !args.no_process {
        let chunk_db_dir = format!("{}/tests", constants::GITHUB_CHUNKS_DB_DIR);
        process_markdown_file(args, &output_filename, &repo_name, &chunk_db_dir).await?;
    }

    Ok(())
}

async fn handle_src_dump(args: &GithubArgs) -> Result<()> {
    info!(
        "Starting GitHub SRC ingestion for URL: {} with version: {:?}",
        args.url, args.version
    );
    println!("📥 Starting source code dump for '{}'...", args.url);

    let task = IngestionTask {
        url: args.url.clone(),
        version: args.version.clone(),
        embedding_api_url: None,
        embedding_model: None,
        embedding_api_key: None,
        extract_included_files: false,
        dump_type: crate::ingest::types::DumpType::Src,
    };

    let crawl_result = Crawler::crawl(&task).await?;
    let repo_name = StorageManager::url_to_repo_name(&args.url);

    println!("📝 Generating consolidated source code file...");

    let ignore_patterns = args.ignore.clone().unwrap_or_default();
    let source_files = Extractor::extract_all_sources(&crawl_result.path, &ignore_patterns)?;

    if source_files.is_empty() {
        println!("No source files were found to generate a markdown file.");
        return Ok(());
    }

    let markdown_content = source_files
        .iter()
        .filter_map(|(path, content)| {
            if content.trim().is_empty() {
                return None;
            }
            let language = path
                .extension()
                .and_then(|s| s.to_str())
                .map(|ext| match ext {
                    "rs" => "rust",
                    "toml" => "toml",
                    "json" => "json",
                    "yaml" | "yml" => "yaml",
                    "md" => "markdown",
                    "js" => "javascript",
                    "ts" => "typescript",
                    "py" => "python",
                    "go" => "go",
                    "java" => "java",
                    "kt" => "kotlin",
                    "swift" => "swift",
                    "sh" => "shell",
                    "rb" => "ruby",
                    "php" => "php",
                    "html" => "html",
                    "css" => "css",
                    _ => "text",
                })
                .unwrap_or("text");

            Some(format!(
                "## `{}`\n\n```{}\n{}\n```\n",
                path.to_string_lossy(),
                language,
                content
            ))
        })
        .collect::<Vec<String>>()
        .join("---\n");

    let safe_version = crawl_result.version.replace('/', "-");
    let output_filename = format!("{repo_name}-{safe_version}-src.md");
    fs::write(&output_filename, markdown_content)?;
    println!("✅ Successfully generated source file: '{output_filename}'");

    if !args.no_process {
        let chunk_db_dir = format!("{}/src", constants::GITHUB_CHUNKS_DB_DIR);
        process_markdown_file(args, &output_filename, &repo_name, &chunk_db_dir).await?;
    }

    Ok(())
}

/// Helper function to process a generated markdown file into a chunked database.
async fn process_markdown_file(
    args: &GithubArgs,
    output_filename: &str,
    repo_name: &str,
    chunk_db_dir: &str,
) -> Result<()> {
    println!("🚀 Automatically processing '{output_filename}' into chunks...");
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
        file_path: output_filename.to_string(),
        separator: "---\n".to_string(),
        embedding_config,
    };

    let source_json = serde_json::to_string(&markdown_source)?;
    let ingestor = MarkdownIngestor;
    let result = ingestor.ingest(&source_json, None).await?;
    let count = result.documents_added;

    if count > 0 {
        println!("✅ Successfully ingested {count} chunks into '{chunk_db_path}'.");
    }
    Ok(())
}
