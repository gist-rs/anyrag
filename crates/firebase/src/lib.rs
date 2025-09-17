//! # `anyrag-firebase`: Firebase Firestore Ingestion Plugin
//!
//! This crate provides the logic for ingesting data from Google Firestore as a self-contained
//! plugin for the `anyrag` ecosystem. It implements the `Ingestor` trait from the
//! core `anyrag` library.

use anyhow::anyhow;
use anyrag::ingest::{state_manager, IngestError as AnyragIngestError, IngestionResult, Ingestor};
use anyrag::providers::db::sqlite::SqliteProvider;
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use firestore::{FirestoreDb, FirestoreDocument, FirestoreQueryDirection, FirestoreTimestamp};
use gcloud_sdk::google::firestore::v1 as firestore_v1;
use serde::{Deserialize, Serialize};
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
    #[error("Date parsing error: {0}")]
    DateParse(#[from] chrono::ParseError),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<FirebaseIngestError> for AnyragIngestError {
    fn from(err: FirebaseIngestError) -> Self {
        match err {
            FirebaseIngestError::Database(e) => AnyragIngestError::Database(e),
            _ => AnyragIngestError::Internal(anyhow!(err.to_string())),
        }
    }
}

// --- Data Structures ---

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FirebaseSource {
    pub project_id: String,
    pub collection: String,
    #[serde(default)]
    pub incremental: bool,
    pub timestamp_field: Option<String>,
    pub limit: Option<i32>,
    pub fields: Option<Vec<String>>,
}

// --- Ingestor Implementation ---

pub struct FirebaseIngestor<'a> {
    sqlite_provider: &'a SqliteProvider,
}

impl<'a> FirebaseIngestor<'a> {
    pub fn new(sqlite_provider: &'a SqliteProvider) -> Self {
        Self { sqlite_provider }
    }
}

#[async_trait]
impl<'a> Ingestor for FirebaseIngestor<'a> {
    async fn ingest(
        &self,
        source: &str,
        _owner_id: Option<&str>,
    ) -> Result<IngestionResult, AnyragIngestError> {
        let firebase_source: FirebaseSource =
            serde_json::from_str(source).map_err(|e| AnyragIngestError::Parse(e.to_string()))?;
        let collection_name = firebase_source.collection.clone();

        let documents_added = dump_firestore_collection(self.sqlite_provider, firebase_source)
            .await
            .map_err(FirebaseIngestError::from)?;

        Ok(IngestionResult {
            documents_added,
            source: collection_name,
            ..Default::default()
        })
    }
}

// --- Core Logic ---

async fn dump_firestore_collection(
    sqlite_provider: &SqliteProvider,
    options: FirebaseSource,
) -> Result<usize, FirebaseIngestError> {
    if Path::new("gcp_creds.json").exists() {
        info!("Setting GOOGLE_APPLICATION_CREDENTIALS to use gcp_creds.json");
        std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", "gcp_creds.json");
    }

    let firestore_db = FirestoreDb::new(&options.project_id).await?;
    let table_name = sanitize_table_name(&options.collection);

    let query_builder = firestore_db.fluent().select();
    let mut query = match &options.fields {
        Some(fields) if !fields.is_empty() => query_builder
            .fields(fields)
            .from(options.collection.as_str()),
        _ => query_builder.from(options.collection.as_str()),
    };

    let last_timestamp = if options.incremental {
        state_manager::read_last_timestamp(&options.project_id, &table_name)
            .map_err(|e| FirebaseIngestError::Internal(e.to_string()))?
            .map(|s| s.parse::<DateTime<Utc>>())
            .transpose()?
            .map(FirestoreTimestamp)
    } else {
        None
    };

    if let (Some(ts_field), Some(last_ts)) = (options.timestamp_field.as_deref(), &last_timestamp) {
        query = query.filter(|q| q.for_all([q.field(ts_field).greater_than(last_ts.clone())]));
    }

    if let Some(ts_field) = options.timestamp_field.as_deref() {
        query = query.order_by([(ts_field, FirestoreQueryDirection::Ascending)]);
    }

    if let Some(limit) = options.limit {
        query = query.limit(limit as u32);
    }

    let mut documents_to_process: Vec<FirestoreDocument> = Vec::new();
    let mut newest_timestamp_seen: Option<FirestoreTimestamp> = last_timestamp;
    let mut stream = query.stream_query_with_errors().await?;

    while let Some(doc) = stream.try_next().await? {
        if let Some(ts_field) = options.timestamp_field.as_deref() {
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
        return Ok(0);
    }

    let schema = infer_schema_from_documents(&documents_to_process)?;
    create_sqlite_table(sqlite_provider, &table_name, &schema, options.incremental).await?;
    insert_documents(sqlite_provider, &table_name, &schema, &documents_to_process).await?;

    if options.incremental {
        if let Some(ts_to_save) = newest_timestamp_seen {
            state_manager::write_last_timestamp(
                &options.project_id,
                &table_name,
                &ts_to_save.0.to_rfc3339(),
            )
            .map_err(|e| FirebaseIngestError::Internal(e.to_string()))?;
        }
    }

    Ok(processed_count)
}

// --- Helper Functions ---

fn to_snake_case(s: &str) -> String {
    let mut snake = String::new();
    let mut chars = s.chars().enumerate().peekable();
    while let Some((i, ch)) = chars.next() {
        if i > 0 && ch.is_uppercase() {
            let prev = s.chars().nth(i - 1).unwrap();
            if prev.is_lowercase() {
                snake.push('_');
            } else if let Some(&(_, next_ch)) = chars.peek() {
                if next_ch.is_lowercase() && prev.is_uppercase() {
                    snake.push('_');
                }
            }
        }
        snake.push(ch.to_ascii_lowercase());
    }
    snake
}

fn get_timestamp_from_doc(doc: &FirestoreDocument, ts_field: &str) -> Option<FirestoreTimestamp> {
    doc.fields
        .get(ts_field)
        .and_then(|val| val.value_type.as_ref())
        .and_then(|vt| match vt {
            firestore_v1::value::ValueType::TimestampValue(ts) => Utc
                .timestamp_opt(ts.seconds, ts.nanos as u32)
                .single()
                .map(FirestoreTimestamp),
            _ => None,
        })
}

pub fn sanitize_table_name(collection_name: &str) -> String {
    collection_name
        .replace(['"', '.'], "")
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

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

fn firestore_type_to_sqlite_type(value: &firestore_v1::Value) -> &'static str {
    match &value.value_type {
        Some(firestore_v1::value::ValueType::IntegerValue(_)) => "INTEGER",
        Some(firestore_v1::value::ValueType::DoubleValue(_)) => "REAL",
        Some(firestore_v1::value::ValueType::BooleanValue(_)) => "INTEGER",
        _ => "TEXT",
    }
}

async fn create_sqlite_table(
    provider: &SqliteProvider,
    table_name: &str,
    schema: &HashMap<String, &'static str>,
    is_incremental: bool,
) -> Result<(), FirebaseIngestError> {
    let conn = provider.db.connect()?;
    if !is_incremental {
        conn.execute(&format!("DROP TABLE IF EXISTS \"{table_name}\";"), ())
            .await?;
    }
    let mut columns_def: Vec<String> = schema
        .iter()
        .map(|(name, dtype)| format!("\"{}\" {}", to_snake_case(name), dtype))
        .collect();
    columns_def.sort();
    columns_def.insert(0, "\"_id\" TEXT PRIMARY KEY".to_string());
    let create_sql = format!(
        "CREATE TABLE IF NOT EXISTS \"{table_name}\" ({});",
        columns_def.join(", ")
    );
    conn.execute(&create_sql, ()).await?;
    Ok(())
}

async fn insert_documents(
    provider: &SqliteProvider,
    table_name: &str,
    schema: &HashMap<String, &'static str>,
    documents: &[FirestoreDocument],
) -> Result<(), FirebaseIngestError> {
    let conn = provider.db.connect()?;
    conn.execute("BEGIN TRANSACTION", ()).await?;
    let mut column_map: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    for camel_case_name in schema.keys() {
        column_map.insert(to_snake_case(camel_case_name), camel_case_name.clone());
    }
    let snake_case_columns: Vec<String> = column_map.keys().cloned().collect();
    let columns_list = snake_case_columns
        .iter()
        .map(|c| format!("\"{c}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let values_placeholders = (0..snake_case_columns.len() + 1)
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");
    let update_set_clause = snake_case_columns
        .iter()
        .map(|c| format!("\"{c}\" = excluded.\"{c}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let insert_sql = format!(
        "INSERT INTO \"{table_name}\" (_id, {columns_list}) VALUES ({values_placeholders})
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
        for snake_case_name in &snake_case_columns {
            let camel_case_name = column_map.get(snake_case_name).unwrap();
            let firestore_value = doc.fields.get(camel_case_name);
            params.push(convert_firestore_value_to_turso(firestore_value.cloned())?);
        }
        stmt.execute(params).await?;
    }
    conn.execute("COMMIT", ()).await?;
    Ok(())
}

fn convert_firestore_value_to_turso(
    firestore_value: Option<firestore_v1::Value>,
) -> Result<TursoValue, FirebaseIngestError> {
    Ok(match firestore_value.and_then(|v| v.value_type) {
        Some(vt) => match vt {
            firestore_v1::value::ValueType::StringValue(s) => TursoValue::Text(s),
            firestore_v1::value::ValueType::IntegerValue(i) => TursoValue::Integer(i),
            firestore_v1::value::ValueType::DoubleValue(d) => TursoValue::Real(d),
            firestore_v1::value::ValueType::BooleanValue(b) => {
                TursoValue::Integer(if b { 1 } else { 0 })
            }
            firestore_v1::value::ValueType::TimestampValue(ts) => {
                let dt = Utc
                    .timestamp_opt(ts.seconds, ts.nanos as u32)
                    .single()
                    .ok_or_else(|| {
                        FirebaseIngestError::Internal("Invalid timestamp".to_string())
                    })?;
                TursoValue::Text(dt.to_rfc3339())
            }
            firestore_v1::value::ValueType::MapValue(_)
            | firestore_v1::value::ValueType::ArrayValue(_) => {
                let serde_val = gcp_value_to_serde_value(firestore_v1::Value {
                    value_type: Some(vt),
                })?;
                TursoValue::Text(serde_json::to_string(&serde_val)?)
            }
            firestore_v1::value::ValueType::NullValue(_) => TursoValue::Null,
            _ => TursoValue::Null,
        },
        None => TursoValue::Null,
    })
}

fn gcp_value_to_serde_value(
    gcp_val: firestore_v1::Value,
) -> Result<serde_json::Value, FirebaseIngestError> {
    Ok(match gcp_val.value_type {
        Some(vt) => match vt {
            firestore_v1::value::ValueType::StringValue(s) => serde_json::Value::String(s),
            firestore_v1::value::ValueType::IntegerValue(i) => serde_json::Value::Number(i.into()),
            firestore_v1::value::ValueType::DoubleValue(d) => serde_json::Number::from_f64(d)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            firestore_v1::value::ValueType::BooleanValue(b) => serde_json::Value::Bool(b),
            firestore_v1::value::ValueType::TimestampValue(ts) => {
                let dt = Utc
                    .timestamp_opt(ts.seconds, ts.nanos as u32)
                    .single()
                    .ok_or_else(|| {
                        FirebaseIngestError::Internal("Invalid timestamp".to_string())
                    })?;
                serde_json::Value::String(dt.to_rfc3339())
            }
            firestore_v1::value::ValueType::MapValue(mv) => {
                let map = mv
                    .fields
                    .into_iter()
                    .map(|(k, v)| gcp_value_to_serde_value(v).map(|v_s| (k, v_s)))
                    .collect::<Result<serde_json::Map<_, _>, _>>()?;
                serde_json::Value::Object(map)
            }
            firestore_v1::value::ValueType::ArrayValue(av) => {
                let arr = av
                    .values
                    .into_iter()
                    .map(gcp_value_to_serde_value)
                    .collect::<Result<Vec<_>, _>>()?;
                serde_json::Value::Array(arr)
            }
            firestore_v1::value::ValueType::NullValue(_) => serde_json::Value::Null,
            _ => serde_json::Value::Null,
        },
        None => serde_json::Value::Null,
    })
}
