use anyhow::{anyhow, bail, Result};
use anyrag::{constants, ingest::Ingestor, providers::db::sqlite::SqliteProvider};
use anyrag_firebase::{FirebaseIngestor, FirebaseSource};
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
            .map_err(|e| anyhow!("Failed to parse gcp_creds.json: {e}"))?;
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

impl FirebaseArgs {
    /// Creates a `FirebaseSource` from the command-line arguments.
    fn to_firebase_source(&self, project_id: String) -> FirebaseSource {
        FirebaseSource {
            project_id,
            collection: self.collection.clone(),
            incremental: self.incremental,
            timestamp_field: self.timestamp_field.clone(),
            limit: self.limit,
            fields: self.fields.clone(),
        }
    }
}

pub async fn handle_dump_firebase(args: &FirebaseArgs) -> Result<()> {
    let project_id = resolve_project_id(args.project_id.as_deref())?;
    let db_path = format!("{}/{project_id}.db", constants::DB_DIR);
    fs::create_dir_all(constants::DB_DIR)?;

    let sqlite_provider = SqliteProvider::new(&db_path).await?;
    sqlite_provider.initialize_schema().await?;
    info!("Local database at '{db_path}' is ready.");

    let firebase_source = args.to_firebase_source(project_id);
    let source_str = serde_json::to_string(&firebase_source)
        .map_err(|e| anyhow!("Failed to serialize Firebase source: {e}"))?;

    let ingestor = FirebaseIngestor::new(&sqlite_provider);
    let result = ingestor
        .ingest(&source_str, None)
        .await
        .map_err(|e| anyhow!("Firebase ingestion failed: {e}"))?;

    info!(
        "Successfully ingested {} new documents from collection '{}'.",
        result.documents_added, result.source
    );

    Ok(())
}
