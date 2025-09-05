//! # anyrag-cli: A CLI for `anyrag`
//!
//! This is the main entry point for the `anyrag` command-line interface.

mod auth;
mod state_manager;

use crate::state_manager::{read_last_timestamp, write_last_timestamp};
use anyhow::{bail, Result};
use anyrag::providers::db::sqlite::SqliteProvider;
use clap::{Parser, Subcommand};

use chrono::{DateTime, Utc};
use firestore::{FirestoreDb, FirestoreDocument, FirestoreQueryDirection, FirestoreTimestamp};
use keyring::Entry;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use tokio_stream::StreamExt;
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
    Process(ProcessArgs),
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
    Firebase(FirebaseArgs),
}

#[derive(Parser, Debug)]
struct FirebaseArgs {
    /// The Google Cloud Project ID for Firestore. If omitted, it will be inferred from `gcp_creds.json`.
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
}

#[derive(Parser, Debug)]
struct ProcessArgs {}

#[derive(Parser, Debug)]
struct ListArgs {
    /// The name of the table to list items from
    table_name: String,
}

#[derive(Parser, Debug)]
struct CountArgs {
    /// The name of the table to count items in
    table_name: String,
}

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
                    eprintln!("Login failed: {e}");
                }
            }
        }
        Commands::Dump(args) => {
            if let Err(e) = handle_dump(args).await {
                eprintln!("Dump failed: {e}");
            }
        }
        Commands::Process(args) => {
            if let Err(e) = handle_process(args).await {
                eprintln!("Process failed: {e}");
            }
        }
        Commands::List(args) => {
            if let Err(e) = handle_list(args).await {
                eprintln!("List command failed: {e}");
            }
        }
        Commands::Count(args) => {
            if let Err(e) = handle_count(args).await {
                eprintln!("Count command failed: {e}");
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
    // 1. Resolve Project ID and set up authentication
    let project_id = resolve_project_id(args.project_id.as_deref())?;

    if Path::new("gcp_creds.json").exists() {
        println!("Found gcp_creds.json, using service account for authentication.");
        info!("Setting GOOGLE_APPLICATION_CREDENTIALS to use gcp_creds.json");
        std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", "gcp_creds.json");
    } else {
        println!("gcp_creds.json not found, falling back to Application Default Credentials.");
        info!(
            "Using Application Default Credentials (run `gcloud auth application-default login`)."
        );
    }

    // 2. Setup clients
    let db_path = "db/anyrag.db";
    let sqlite_provider = SqliteProvider::new(db_path).await?;
    sqlite_provider.initialize_schema().await?;
    info!("Local database at '{db_path}' is ready.");

    let firestore_db = FirestoreDb::new(&project_id).await?;

    // 3. Build Firestore Request
    let table_name = sanitize_table_name(&args.collection);

    let mut query = firestore_db
        .fluent()
        .select()
        .from(args.collection.as_str());

    let last_timestamp = if args.incremental {
        read_last_timestamp(&project_id, &table_name)?
            .map(|s| s.parse::<DateTime<Utc>>())
            .transpose()?
            .map(FirestoreTimestamp)
    } else {
        None
    };

    if let (Some(ts_field), Some(last_ts)) = (&args.timestamp_field, &last_timestamp) {
        info!(
            "Incremental sync: fetching documents where {} > {}",
            ts_field, last_ts.0
        );
        query = query.filter(|q| q.for_all([q.field(ts_field).greater_than(last_ts.clone())]));
    }

    if let Some(ts_field) = &args.timestamp_field {
        query = query.order_by([(ts_field.as_str(), FirestoreQueryDirection::Ascending)]);
    }

    if let Some(limit) = args.limit {
        query = query.limit(limit as u32);
    }

    // 4. Fetch Documents
    println!("Connecting to Firestore and fetching documents...");
    let mut documents_to_process: Vec<FirestoreDocument> = Vec::new();
    let mut newest_timestamp_seen: Option<FirestoreTimestamp> = last_timestamp;

    let mut stream = query.stream_query_with_errors().await?;
    while let Some(doc) = stream.try_next().await? {
        if let Some(ts_field) = &args.timestamp_field {
            if let Some(doc_ts) = get_timestamp_from_doc(&doc, ts_field) {
                if newest_timestamp_seen.is_none()
                    || doc_ts.0 > newest_timestamp_seen.as_ref().unwrap().0
                {
                    newest_timestamp_seen = Some(doc_ts);
                }
            }
        }
        documents_to_process.push(doc);
    }

    if documents_to_process.is_empty() {
        println!(
            "No new documents found in collection '{}'.",
            args.collection
        );
        return Ok(());
    }
    println!(
        "Fetched {} new or updated documents. Writing to local DB...",
        documents_to_process.len()
    );

    // 5. Schema, Table, and Insertion
    let schema = infer_schema_from_documents(&documents_to_process)?;
    create_sqlite_table(&sqlite_provider, &table_name, &schema, args.incremental).await?;
    insert_documents(
        &sqlite_provider,
        &table_name,
        &schema,
        &documents_to_process,
    )
    .await?;

    // 6. Update State
    if args.incremental {
        if let Some(ts_to_save) = newest_timestamp_seen {
            write_last_timestamp(&project_id, &table_name, &ts_to_save.0.to_rfc3339())?;
        }
    }

    println!(
        "✅ Successfully wrote {} documents to table '{table_name}'.",
        documents_to_process.len()
    );

    Ok(())
}

fn resolve_project_id(project_id_arg: Option<&str>) -> Result<String> {
    if let Some(id) = project_id_arg {
        return Ok(id.to_string());
    }

    if let Ok(file_content) = std::fs::read_to_string("gcp_creds.json") {
        let json: serde_json::Value = serde_json::from_str(&file_content)?;
        if let Some(project_id) = json["project_id"].as_str() {
            println!("Inferred project ID '{project_id}' from gcp_creds.json.");
            return Ok(project_id.to_string());
        }
    }

    bail!("Project ID not provided and could not be inferred from gcp_creds.json. Please use the --project-id flag.")
}

// --- Helper Functions for Firebase Dump ---

fn get_timestamp_from_doc(doc: &FirestoreDocument, ts_field: &str) -> Option<FirestoreTimestamp> {
    use chrono::TimeZone;
    doc.fields
        .get(ts_field)
        .and_then(|val| val.value_type.as_ref())
        .and_then(|vt| match vt {
            gcloud_sdk::google::firestore::v1::value::ValueType::TimestampValue(ts) => Utc
                .timestamp_opt(ts.seconds, ts.nanos as u32)
                .single()
                .map(FirestoreTimestamp),
            _ => None,
        })
}

/// Sanitizes a collection name to be a valid SQLite table name.
fn sanitize_table_name(collection_name: &str) -> String {
    collection_name
        .replace('"', "")
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

/// Infers a SQLite schema from a slice of Firestore documents.
fn infer_schema_from_documents(
    documents: &[FirestoreDocument],
) -> Result<HashMap<String, &'static str>> {
    let mut schema = HashMap::new();
    for doc in documents {
        for (field_name, gcp_value) in &doc.fields {
            let sqlite_type = firestore_type_to_sqlite_type(gcp_value);
            schema.entry(field_name.clone()).or_insert(sqlite_type);
        }
    }
    Ok(schema)
}

/// Maps a Firestore value type to a SQLite column type.
fn firestore_type_to_sqlite_type(value: &gcloud_sdk::google::firestore::v1::Value) -> &'static str {
    match &value.value_type {
        Some(gcloud_sdk::google::firestore::v1::value::ValueType::IntegerValue(_)) => "INTEGER",
        Some(gcloud_sdk::google::firestore::v1::value::ValueType::DoubleValue(_)) => "REAL",
        Some(gcloud_sdk::google::firestore::v1::value::ValueType::BooleanValue(_)) => "INTEGER",
        _ => "TEXT",
    }
}

/// Creates a table in the SQLite database. If not in incremental mode, it drops the table first.
async fn create_sqlite_table(
    provider: &SqliteProvider,
    table_name: &str,
    schema: &HashMap<String, &'static str>,
    is_incremental: bool,
) -> Result<()> {
    let conn = provider.db.connect()?;

    if !is_incremental {
        conn.execute(&format!("DROP TABLE IF EXISTS {table_name};"), ())
            .await?;
        info!("Dropped existing table '{}' for full refresh.", table_name);
    }

    let mut columns_def: Vec<String> = schema
        .iter()
        .map(|(name, dtype)| format!("\"{name}\" {dtype}"))
        .collect();
    columns_def.sort();
    columns_def.insert(0, "\"_id\" TEXT PRIMARY KEY".to_string());

    let create_sql = format!(
        "CREATE TABLE IF NOT EXISTS {} ({});",
        table_name,
        columns_def.join(", ")
    );
    info!("Executing CREATE TABLE: {}", create_sql);
    conn.execute(&create_sql, ()).await?;

    Ok(())
}

/// Inserts or updates Firestore documents into the specified SQLite table using ON CONFLICT.
async fn insert_documents(
    provider: &SqliteProvider,
    table_name: &str,
    schema: &HashMap<String, &'static str>,
    documents: &[FirestoreDocument],
) -> Result<()> {
    let conn = provider.db.connect()?;
    conn.execute("BEGIN TRANSACTION", ()).await?;

    let mut columns: Vec<String> = schema.keys().cloned().collect();
    columns.sort();

    let columns_list = columns
        .iter()
        .map(|c| format!("\"{c}\""))
        .collect::<Vec<_>>()
        .join(", ");

    let values_placeholders = (0..columns.len() + 1)
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");

    let update_set_clause = columns
        .iter()
        .map(|c| format!("\"{c}\" = excluded.\"{c}\""))
        .collect::<Vec<_>>()
        .join(", ");

    let insert_sql = format!(
        "INSERT INTO {table_name} (_id, {columns_list}) VALUES ({values_placeholders})
         ON CONFLICT(_id) DO UPDATE SET {update_set_clause};"
    );

    let mut stmt = conn.prepare(&insert_sql).await?;

    for doc in documents {
        let doc_id = doc
            .name
            .split('/')
            .next_back()
            .unwrap_or_default()
            .to_string();
        let mut params: Vec<TursoValue> = vec![doc_id.into()];

        for col_name in &columns {
            let firestore_value = doc.fields.get(col_name);
            let turso_val = convert_firestore_value_to_turso(firestore_value.cloned())?;
            params.push(turso_val);
        }

        stmt.execute(params).await?;
    }

    conn.execute("COMMIT", ()).await?;
    Ok(())
}

/// Converts a GCP Firestore Value into a `serde_json::Value`.
fn gcp_value_to_serde_value(
    gcp_val: gcloud_sdk::google::firestore::v1::Value,
) -> Result<serde_json::Value> {
    use gcloud_sdk::google::firestore::v1::value::ValueType;
    let serde_val = match gcp_val.value_type {
        Some(vt) => match vt {
            ValueType::StringValue(s) => serde_json::Value::String(s),
            ValueType::IntegerValue(i) => serde_json::Value::Number(i.into()),
            ValueType::DoubleValue(d) => {
                serde_json::Value::Number(serde_json::Number::from_f64(d).unwrap())
            }
            ValueType::BooleanValue(b) => serde_json::Value::Bool(b),
            ValueType::TimestampValue(ts) => {
                use chrono::TimeZone;
                let dt = Utc
                    .timestamp_opt(ts.seconds, ts.nanos as u32)
                    .single()
                    .ok_or_else(|| anyhow::anyhow!("Invalid timestamp"))?;
                serde_json::Value::String(dt.to_rfc3339())
            }
            ValueType::MapValue(mv) => {
                let mut map = serde_json::Map::new();
                for (k, v) in mv.fields {
                    map.insert(k, gcp_value_to_serde_value(v)?);
                }
                serde_json::Value::Object(map)
            }
            ValueType::ArrayValue(av) => {
                let mut arr = Vec::new();
                for v in av.values {
                    arr.push(gcp_value_to_serde_value(v)?);
                }
                serde_json::Value::Array(arr)
            }
            ValueType::NullValue(_) => serde_json::Value::Null,
            _ => serde_json::Value::Null, // Bytes, GeoPoint, etc. are not handled
        },
        None => serde_json::Value::Null,
    };
    Ok(serde_val)
}

/// Converts a Firestore `Value` to a `turso::Value`.
fn convert_firestore_value_to_turso(
    firestore_value: Option<gcloud_sdk::google::firestore::v1::Value>,
) -> Result<TursoValue> {
    use gcloud_sdk::google::firestore::v1::value::ValueType;
    let val = match firestore_value.and_then(|v| v.value_type) {
        Some(vt) => match vt {
            ValueType::StringValue(s) => TursoValue::Text(s),
            ValueType::IntegerValue(i) => TursoValue::Integer(i),
            ValueType::DoubleValue(d) => TursoValue::Real(d),
            ValueType::BooleanValue(b) => TursoValue::Integer(if b { 1 } else { 0 }),
            ValueType::TimestampValue(ts) => {
                use chrono::TimeZone;
                let dt = Utc
                    .timestamp_opt(ts.seconds, ts.nanos as u32)
                    .single()
                    .ok_or_else(|| anyhow::anyhow!("Invalid timestamp"))?;
                TursoValue::Text(dt.to_rfc3339())
            }
            ValueType::MapValue(mv) => {
                let serde_val =
                    gcp_value_to_serde_value(gcloud_sdk::google::firestore::v1::Value {
                        value_type: Some(ValueType::MapValue(mv)),
                    })?;
                TursoValue::Text(serde_json::to_string(&serde_val)?)
            }
            ValueType::ArrayValue(av) => {
                let serde_val =
                    gcp_value_to_serde_value(gcloud_sdk::google::firestore::v1::Value {
                        value_type: Some(ValueType::ArrayValue(av)),
                    })?;
                TursoValue::Text(serde_json::to_string(&serde_val)?)
            }
            ValueType::NullValue(_) => TursoValue::Null,
            _ => TursoValue::Null, // For Bytes, Reference, GeoPoint
        },
        None => TursoValue::Null,
    };
    Ok(val)
}

async fn handle_process(_args: &ProcessArgs) -> Result<()> {
    println!("Processing local data...");
    bail!("Processing not yet implemented.");
}

async fn handle_list(args: &ListArgs) -> Result<()> {
    let db_path = "db/anyrag.db";
    if !Path::new(db_path).exists() {
        bail!("Database file not found. Run a `dump` command first.");
    }
    let sqlite_provider = SqliteProvider::new(db_path).await?;
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

    // Print headers
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
    let db_path = "db/anyrag.db";
    if !Path::new(db_path).exists() {
        bail!("Database file not found. Run a `dump` command first.");
    }
    let sqlite_provider = SqliteProvider::new(db_path).await?;
    let conn = sqlite_provider.db.connect()?;

    let sql = format!("SELECT COUNT(*) FROM {}", args.table_name);
    let mut stmt = conn.prepare(&sql).await?;
    let mut rows = stmt.query(()).await?;

    if let Some(row) = rows.next().await? {
        let count: i64 = row.get(0)?;
        println!("Table '{}' has {} rows.", args.table_name, count);
    } else {
        // This case should be unlikely with COUNT(*) but good to have.
        bail!("Could not count rows in table '{}'.", args.table_name);
    }

    Ok(())
}
