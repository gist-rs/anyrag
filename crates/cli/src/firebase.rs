use anyhow::{bail, Result};
use anyrag::{
    ingest::{dump_firestore_collection, DumpFirestoreOptions},
    providers::db::sqlite::SqliteProvider,
};
use clap::Parser;
use std::fs;
use tracing::info;

/// Resolves the GCP Project ID.
///
/// It prioritizes the explicitly provided ID, then falls back to inferring
/// it from a `gcp_creds.json` file.
pub fn resolve_project_id(project_id_arg: Option<&str>) -> Result<String> {
    if let Some(id) = project_id_arg {
        return Ok(id.to_string());
    }

    if let Ok(file_content) = fs::read_to_string("gcp_creds.json") {
        let json: serde_json::Value = serde_json::from_str(&file_content)
            .map_err(|e| anyhow::anyhow!("Failed to parse gcp_creds.json: {e}"))?;
        if let Some(project_id) = json["project_id"].as_str() {
            println!("Inferred project ID '{project_id}' from gcp_creds.json.");
            return Ok(project_id.to_string());
        }
    }

    bail!("Project ID not provided and could not be inferred from gcp_creds.json. Please use the --project-id flag.")
}

#[derive(Parser, Debug)]
pub struct FirebaseArgs {
    /// The Google Cloud Project ID. If omitted, it will be inferred from `gcp_creds.json`.
    #[arg(long)]
    project_id: Option<String>,
    /// The name of the Firestore collection to dump
    #[arg(long, required = true)]
    collection: String,
    /// Enable incremental sync to fetch only new or updated documents
    #[arg(long)]
    incremental: bool,
    /// The document field for ordering and checkpointing (required for incremental sync)
    #[arg(long, requires = "incremental")]
    timestamp_field: Option<String>,
    /// Limit the number of documents to dump, useful for testing
    #[arg(long)]
    limit: Option<i32>,
    /// Comma-separated list of specific fields to select. If omitted, all fields are dumped.
    #[arg(long, value_delimiter = ',')]
    fields: Option<Vec<String>>,
}

pub async fn handle_dump_firebase(args: &FirebaseArgs) -> Result<()> {
    let project_id = resolve_project_id(args.project_id.as_deref())?;
    let db_path = format!("db/{project_id}.db");
    fs::create_dir_all("db")?;

    let sqlite_provider = SqliteProvider::new(&db_path).await?;
    sqlite_provider.initialize_schema().await?;
    info!("Local database at '{db_path}' is ready.");

    let options = DumpFirestoreOptions {
        project_id: &project_id,
        collection: &args.collection,
        incremental: args.incremental,
        timestamp_field: args.timestamp_field.as_deref(),
        limit: args.limit,
        fields: args.fields.as_deref(),
    };

    dump_firestore_collection(&sqlite_provider, options)
        .await
        .map_err(|e| anyhow::anyhow!("Firebase dump failed: {e}"))?;

    Ok(())
}
