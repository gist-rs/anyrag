//! # anyrag-cli: A CLI for `anyrag`
//!
//! This is the main entry point for the `anyrag` command-line interface.

mod api_client;
mod auth;

use anyhow::{bail, Result};
use anyrag::providers::db::sqlite::SqliteProvider;
use clap::{Parser, Subcommand};
use gcp_sdk::google::firestore::v1::firestore_client::FirestoreClient;
use gcp_sdk::google::firestore::v1::{Document, ListDocumentsRequest};
use gcp_sdk::TokenSource;
use keyring::Entry;
use std::fs::File;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

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
    Process(ProcessArgs),
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
    Firebase(FirebaseArgs),
}

#[derive(Parser, Debug)]
struct FirebaseArgs {
    /// The Google Cloud Project ID for Firestore
    #[arg(long, required = true)]
    project_id: String,
    /// The name of the Firestore collection to dump
    #[arg(long)]
    collection: String,
}

#[derive(Parser, Debug)]
struct ProcessArgs {}

// --- Main Application Entry ---

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging to a file
    let log_file = File::create("anyrag-cli.log")?;
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
                    eprintln!("Login failed: {}", e);
                }
            }
        }
        Commands::Dump(args) => {
            if let Err(e) = handle_dump(args).await {
                eprintln!("Dump failed: {}", e);
            }
        }
        Commands::Process(args) => {
            if let Err(e) = handle_process(args).await {
                eprintln!("Process failed: {}", e);
            }
        }
    }

    Ok(())
}

// --- Command Handlers ---

async fn handle_dump(args: &DumpArgs) -> Result<()> {
    match &args.command {
        DumpCommands::Firebase(firebase_args) => {
            handle_dump_firebase(firebase_args).await?;
        }
    }
    Ok(())
}

async fn handle_dump_firebase(args: &FirebaseArgs) -> Result<()> {
    info!(
        "Dumping from Firebase project: '{}', collection: '{}'",
        args.project_id, args.collection
    );

    // 1. Check for authentication
    let entry = Entry::new("anyrag-cli", "user")?;
    let _token = entry
        .get_password()
        .map_err(|_| anyhow::anyhow!("You are not logged in. Please run `anyrag login` first."))?;

    // 2. Setup local SQLite database
    let sqlite_provider = SqliteProvider::new("db/anyrag.db").await?;
    sqlite_provider.initialize_schema().await?;
    info!("Local database is ready.");

    // 3. Setup GCP authentication
    let gcp_auth = gcp_sdk::TokenSource::ApplicationDefaultCredentials
        .get_token(&[])
        .await?;
    let mut firestore_client = FirestoreClient::new(gcp_auth).await?;

    // 4. Fetch documents from Firestore
    let parent = format!("projects/{}/databases/(default)/documents", args.project_id);
    let request = ListDocumentsRequest {
        parent,
        collection_id: args.collection.clone(),
        ..Default::default()
    };

    let response = firestore_client.list_documents(request).await?;
    let documents = response.documents;

    if documents.is_empty() {
        println!("No documents found in collection '{}'.", args.collection);
        return Ok(());
    }

    println!(
        "Found {} documents. Preparing to write to local DB...",
        documents.len()
    );

    // 5. Write to SQLite (Simplified for now)
    // TODO: Dynamically create table based on document schema
    // TODO: Insert each document
    for doc in documents {
        info!("Processing document: {}", doc.name);
        // This part will be complex, involving mapping Firestore Value types to SQLite types.
        // For now, I'll just log it.
        for (key, value) in doc.fields {
            info!("  - {}: {:?}", key, value.value_type);
        }
    }

    bail!("Data insertion into SQLite not yet implemented.");
}

async fn handle_process(_args: &ProcessArgs) -> Result<()> {
    println!("Processing local data...");
    // TODO: Implement data processing logic
    bail!("Processing not yet implemented.");
}
