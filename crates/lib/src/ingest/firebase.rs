//! # Firebase Firestore Ingestion
//!
//! This module provides the core logic for dumping a Firestore collection
//! to a local SQLite database. It handles authentication, data fetching,
//! schema inference, and writing to SQLite.

use crate::{ingest::state_manager, providers::db::sqlite::SqliteProvider};

use chrono::{DateTime, Utc};
use firestore::{FirestoreDb, FirestoreDocument, FirestoreQueryDirection, FirestoreTimestamp};
use std::{collections::HashMap, path::Path};
use thiserror::Error;
use tokio_stream::StreamExt;
use tracing::info;
use turso::Value as TursoValue;

// --- Error Definitions ---

#[derive(Error, Debug)]
pub enum FirebaseIngestError {
    #[error("Database error: {0}")]
    Database(#[from] turso::Error),
    #[error("Firestore error: {0}")]
    Firestore(#[from] firestore::errors::FirestoreError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Authentication error: {0}")]
    Auth(String),
    #[error("Date parsing error: {0}")]
    DateParse(#[from] chrono::ParseError),
    #[error("Internal error: {0}")]
    Internal(String),
}

// --- Data Structures ---

/// Options for configuring a Firestore dump operation.
pub struct DumpFirestoreOptions<'a> {
    pub project_id: Option<&'a str>,
    pub collection: &'a str,
    pub incremental: bool,
    pub timestamp_field: Option<&'a str>,
    pub limit: Option<i32>,
}

// --- Public API ---

/// Fetches documents from a Firestore collection and stores them in a local SQLite database.
///
/// This function orchestrates the entire dump process, including:
/// - Authenticating with Google Cloud.
/// - Building and executing a Firestore query.
/// - Handling full vs. incremental data fetches.
/// - Inferring a SQLite schema from the Firestore documents.
/// - Creating the destination table and inserting the data.
///
/// # Arguments
/// * `sqlite_provider`: The `SqliteProvider` for the destination database.
/// * `options`: The configuration for the dump operation.
///
/// # Returns
/// The number of documents successfully written to the database.
pub async fn dump_firestore_collection(
    sqlite_provider: &SqliteProvider,
    options: DumpFirestoreOptions<'_>,
) -> Result<usize, FirebaseIngestError> {
    // 1. Resolve Project ID and set up authentication
    let project_id = resolve_project_id(options.project_id)?;

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
    let firestore_db = FirestoreDb::new(&project_id).await?;
    let table_name = sanitize_table_name(options.collection);

    // 3. Build Firestore Request
    let mut query = firestore_db.fluent().select().from(options.collection);

    let last_timestamp = if options.incremental {
        state_manager::read_last_timestamp(&project_id, &table_name)
            .map_err(|e| FirebaseIngestError::Internal(e.to_string()))?
            .map(|s| s.parse::<DateTime<Utc>>())
            .transpose()?
            .map(FirestoreTimestamp)
    } else {
        None
    };

    if let (Some(ts_field), Some(last_ts)) = (options.timestamp_field, &last_timestamp) {
        info!(
            "Incremental sync: fetching documents where {ts_field} > {}",
            last_ts.0
        );
        query = query.filter(|q| q.for_all([q.field(ts_field).greater_than(last_ts.clone())]));
    }

    if let Some(ts_field) = options.timestamp_field {
        query = query.order_by([(ts_field, FirestoreQueryDirection::Ascending)]);
    }

    if let Some(limit) = options.limit {
        query = query.limit(limit as u32);
    }

    // 4. Fetch Documents
    println!("Connecting to Firestore and fetching documents...");
    let mut documents_to_process: Vec<FirestoreDocument> = Vec::new();
    let mut newest_timestamp_seen: Option<FirestoreTimestamp> = last_timestamp;

    let mut stream = query.stream_query_with_errors().await?;
    while let Some(doc) = stream.try_next().await? {
        if let Some(ts_field) = options.timestamp_field {
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

    let processed_count = documents_to_process.len();
    if processed_count == 0 {
        println!(
            "No new documents found in collection '{}'.",
            options.collection
        );
        return Ok(0);
    }
    println!("Fetched {processed_count} new or updated documents. Writing to local DB...",);

    // 5. Schema, Table, and Insertion
    let schema = infer_schema_from_documents(&documents_to_process)?;
    create_sqlite_table(sqlite_provider, &table_name, &schema, options.incremental).await?;
    insert_documents(sqlite_provider, &table_name, &schema, &documents_to_process).await?;

    // 6. Update State
    if options.incremental {
        if let Some(ts_to_save) = newest_timestamp_seen {
            state_manager::write_last_timestamp(
                &project_id,
                &table_name,
                &ts_to_save.0.to_rfc3339(),
            )
            .map_err(|e| FirebaseIngestError::Internal(e.to_string()))?;
        }
    }

    println!("✅ Successfully wrote {processed_count} documents to table '{table_name}'.",);

    Ok(processed_count)
}

// --- Implementation Details (private functions) ---

fn resolve_project_id(project_id_arg: Option<&str>) -> Result<String, FirebaseIngestError> {
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

    Err(FirebaseIngestError::Auth(
        "Project ID not provided and could not be inferred from gcp_creds.json. Please use the --project-id flag."
            .to_string(),
    ))
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
) -> Result<HashMap<String, &'static str>, FirebaseIngestError> {
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
) -> Result<(), FirebaseIngestError> {
    let conn = provider.db.connect()?;

    if !is_incremental {
        conn.execute(&format!("DROP TABLE IF EXISTS {table_name};"), ())
            .await?;
        info!("Dropped existing table '{table_name}' for full refresh.");
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
    info!("Executing CREATE TABLE: {create_sql}");
    conn.execute(&create_sql, ()).await?;
    Ok(())
}

/// Inserts or updates Firestore documents into the specified SQLite table using ON CONFLICT.
async fn insert_documents(
    provider: &SqliteProvider,
    table_name: &str,
    schema: &HashMap<String, &'static str>,
    documents: &[FirestoreDocument],
) -> Result<(), FirebaseIngestError> {
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
) -> Result<serde_json::Value, FirebaseIngestError> {
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
                    .ok_or_else(|| {
                        FirebaseIngestError::Internal("Invalid timestamp".to_string())
                    })?;
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
) -> Result<TursoValue, FirebaseIngestError> {
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
                    .ok_or_else(|| {
                        FirebaseIngestError::Internal("Invalid timestamp".to_string())
                    })?;
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
