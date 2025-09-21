//! # anyrag-cli: A CLI for `anyrag`
//!
//! This is the main entry point for the `anyrag` command-line interface.

mod auth;
mod firebase;
mod process;
use anyhow::{bail, Result};

use anyrag::constants;
use anyrag_github::cli::{handle_dump_github, GithubArgs};
use clap::{Parser, Subcommand};
use keyring::Entry;
use std::fs;
use std::path::Path;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};
use turso::Value as TursoValue;

// --- CLI Definition ---

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Login via Google OAuth2 to authenticate the CLI
    Login(LoginArgs),
    /// Dump data from a remote source to the local database
    Dump(DumpArgs),
    /// Process and enrich data in the local database
    Process(process::ProcessArgs),
    /// List items from a local database table
    List(ListArgs),
    /// Count items in a local database table
    Count(CountArgs),
}

#[derive(Parser, Debug)]
struct LoginArgs {}

#[derive(Parser, Debug)]
struct DumpArgs {
    #[command(subcommand)]
    command: DumpCommands,
}

#[derive(Subcommand, Debug)]
enum DumpCommands {
    /// Dump data from a Google Firestore collection
    Firebase(firebase::FirebaseArgs),
    /// Dump examples from a public GitHub repository
    #[command(disable_version_flag = true)]
    Github(GithubArgs),
}

#[derive(Parser, Debug)]
struct ListArgs {
    /// The Google Cloud Project ID. If omitted, it will be inferred from `gcp_creds.json`.
    #[arg(long)]
    project_id: Option<String>,
    /// The name of the table to list items from
    table_name: String,
}

#[derive(Parser, Debug)]
struct CountArgs {
    /// The Google Cloud Project ID. If omitted, it will be inferred from `gcp_creds.json`.
    #[arg(long)]
    project_id: Option<String>,
    /// The name of the table to count items in
    table_name: String,
}

// --- Main Application Entry ---

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging to a file
    let log_file = fs::File::create("anyrag-cli.log")?;
    let subscriber = fmt::Subscriber::builder()
        .with_writer(log_file)
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let cli = Cli::parse();

    // Handle the command
    match &cli.command {
        Commands::Login(_) => {
            info!("Starting login process...");
            match auth::login().await {
                Ok(token) => {
                    let entry = Entry::new("anyrag-cli", "user")?;
                    entry.set_password(&token)?;
                    info!("Login successful. Token stored securely.");
                    println!("✅ Login successful!");
                }
                Err(e) => {
                    eprintln!("Login failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Dump(args) => {
            if let Err(e) = handle_dump(args).await {
                eprintln!("Dump failed: {e}");
                std::process::exit(1);
            }
        }
        Commands::Process(args) => {
            if let Err(e) = handle_process(args).await {
                eprintln!("Process failed: {e}");
                std::process::exit(1);
            }
        }
        Commands::List(args) => {
            if let Err(e) = handle_list(args).await {
                eprintln!("List command failed: {e}");
                std::process::exit(1);
            }
        }
        Commands::Count(args) => {
            if let Err(e) = handle_count(args).await {
                eprintln!("Count command failed: {e}");
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

// --- Command Handlers ---

async fn handle_dump(args: &DumpArgs) -> Result<()> {
    match &args.command {
        DumpCommands::Firebase(firebase_args) => {
            firebase::handle_dump_firebase(firebase_args).await?;
        }
        DumpCommands::Github(github_args) => {
            handle_dump_github(github_args).await?;
        }
    }
    Ok(())
}

async fn handle_process(args: &process::ProcessArgs) -> Result<()> {
    process::handle_process(args).await
}

async fn handle_list(args: &ListArgs) -> Result<()> {
    let project_id = firebase::resolve_project_id(args.project_id.as_deref())?;
    let db_path = format!("{}/{project_id}.db", constants::DB_DIR);
    if !Path::new(&db_path).exists() {
        bail!("Database file '{db_path}' not found. Run a `dump` command first for project '{project_id}'.");
    }
    let sqlite_provider = anyrag::providers::db::sqlite::SqliteProvider::new(&db_path).await?;
    let conn = sqlite_provider.db.connect()?;

    let sql = format!("SELECT * FROM {} LIMIT 10", args.table_name);
    let mut stmt = conn.prepare(&sql).await?;
    let column_names: Vec<String> = stmt
        .columns()
        .iter()
        .map(|c| c.name().to_string())
        .collect();

    if column_names.is_empty() {
        println!(
            "Table '{}' has no columns or does not exist.",
            args.table_name
        );
        return Ok(());
    }

    let headers = column_names.join(" | ");
    println!("{headers}");
    println!("{}", "-".repeat(headers.len()));

    let mut rows = stmt.query(()).await?;
    let mut row_count = 0;
    while let Some(row) = rows.next().await? {
        row_count += 1;
        let values: Vec<String> = (0..column_names.len())
            .map(|i| {
                let val = row.get_value(i).unwrap_or(TursoValue::Null);
                let mut s = match val {
                    TursoValue::Text(s) => s,
                    TursoValue::Integer(i) => i.to_string(),
                    TursoValue::Real(f) => f.to_string(),
                    TursoValue::Blob(_) => "[BLOB]".to_string(),
                    TursoValue::Null => "NULL".to_string(),
                };
                if s.len() > 50 {
                    s.truncate(47);
                    s.push_str("...");
                }
                s
            })
            .collect();
        println!("{}", values.join(" | "));
    }

    if row_count == 0 {
        println!("No rows found in table '{}'.", args.table_name);
    }

    Ok(())
}

async fn handle_count(args: &CountArgs) -> Result<()> {
    let project_id = firebase::resolve_project_id(args.project_id.as_deref())?;
    let db_path = format!("{}/{project_id}.db", constants::DB_DIR);
    if !Path::new(&db_path).exists() {
        bail!("Database file '{db_path}' not found. Run a `dump` command first for project '{project_id}'.");
    }
    let sqlite_provider = anyrag::providers::db::sqlite::SqliteProvider::new(&db_path).await?;
    let conn = sqlite_provider.db.connect()?;

    let sql = format!("SELECT COUNT(*) FROM {}", args.table_name);
    let mut stmt = conn.prepare(&sql).await?;
    let mut rows = stmt.query(()).await?;

    if let Some(row) = rows.next().await? {
        let count: i64 = row.get(0)?;
        println!("Table '{}' has {} rows.", args.table_name, count);
    } else {
        bail!("Could not count rows in table '{}'.", args.table_name);
    }

    Ok(())
}
