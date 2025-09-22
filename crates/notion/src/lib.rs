//! # `anyrag-notion`: Notion Ingestion Plugin
//!
//! This crate provides the logic for ingesting data from Notion databases as a self-contained
//! plugin for the `anyrag` ecosystem. It implements the `Ingestor` trait from the
//! core `anyrag` library.

use anyhow::anyhow;
use anyrag::ingest::traits::{IngestError, IngestionResult, Ingestor};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::env;
use thiserror::Error;
use tracing::{info, warn};
use turso::{params, Connection, Database, Value};

// --- Error Definitions ---

#[derive(Error, Debug, Clone)]
pub enum NotionError {
    #[error("Invalid Notion source JSON: {0}")]
    InvalidSource(String),
    #[error("Failed to fetch from Notion API: {0}")]
    Fetch(String),
    #[error("Notion API returned an error: {0}")]
    ApiError(String),
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),
    #[error("No data sources found for the given database")]
    NoDataSource,
}

impl From<reqwest::Error> for NotionError {
    fn from(err: reqwest::Error) -> Self {
        NotionError::Fetch(err.to_string())
    }
}

/// A helper to convert the specific `NotionError` into the generic `anyrag::ingest::IngestError`.
impl From<NotionError> for IngestError {
    fn from(err: NotionError) -> Self {
        match err {
            NotionError::InvalidSource(msg) => IngestError::Parse(msg),
            NotionError::Fetch(msg) => IngestError::Fetch(msg),
            NotionError::ApiError(msg) => IngestError::Internal(anyhow!(msg)),
            NotionError::MissingEnvVar(msg) => {
                IngestError::Internal(anyhow!("Missing environment variable: {}", msg))
            }
            NotionError::NoDataSource => {
                IngestError::SourceNotFound("No data sources found for database".into())
            }
        }
    }
}

// --- Notion API Response Structures ---

#[derive(Deserialize, Debug, Clone)]
struct PlainText {
    plain_text: String,
}

#[derive(Deserialize, Debug, Clone)]
struct DateValue {
    start: String,
    end: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
enum PropertyValue {
    Title {
        title: Vec<PlainText>,
    },
    RichText {
        rich_text: Vec<PlainText>,
    },
    Date {
        date: Option<DateValue>,
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize, Debug)]
struct Page {
    id: String,
    properties: HashMap<String, PropertyValue>,
}

#[derive(Deserialize, Debug)]
struct QueryResponse {
    results: Vec<Page>,
    next_cursor: Option<String>,
    has_more: bool,
}

#[derive(Deserialize, Debug)]
struct DataSource {
    id: String,
}

#[derive(Deserialize, Debug)]
struct DatabaseResponse {
    id: String,
    data_sources: Vec<DataSource>,
}

// --- Ingestor Implementation ---

/// Defines the structure of the JSON string passed to the `ingest` method.
#[derive(Deserialize)]
struct NotionSource {
    database_id: String,
}

/// The `Ingestor` implementation for Notion.
pub struct NotionIngestor<'a> {
    db: &'a Database,
}

impl<'a> NotionIngestor<'a> {
    /// Creates a new `NotionIngestor`.
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl Ingestor for NotionIngestor<'_> {
    /// Ingests a Notion Database.
    ///
    /// The `source` argument is expected to be a JSON string with a `database_id` key,
    /// for example:
    /// `{"database_id": "276fdc98-..."}`.
    async fn ingest(
        &self,
        source: &str,
        _owner_id: Option<&str>,
    ) -> Result<IngestionResult, IngestError> {
        let notion_source: NotionSource =
            serde_json::from_str(source).map_err(|e| NotionError::InvalidSource(e.to_string()))?;
        let db_id = notion_source.database_id;

        info!("Starting ingestion for Notion database: {}", db_id);

        let notion_token = env::var("NOTION_TOKEN")
            .map_err(|_| NotionError::MissingEnvVar("NOTION_TOKEN".into()))?;
        let notion_version = env::var("NOTION_VERSION")
            .map_err(|_| NotionError::MissingEnvVar("NOTION_VERSION".into()))?;

        let client = reqwest::Client::new();
        let headers = construct_headers(&notion_token, &notion_version)?;

        // 1. Get database info to find the data_source_id.
        let db_info = fetch_database_info(&client, &headers, &db_id).await?;
        let data_source_id = db_info
            .data_sources
            .first()
            .ok_or(NotionError::NoDataSource)?
            .id
            .clone();
        info!("Found data source ID: {}", data_source_id);

        // 2. Query the data source to get all pages.
        let pages = query_all_pages(&client, &headers, &data_source_id).await?;
        let pages_count = pages.len();
        info!("Fetched {} pages from Notion.", pages_count);

        if pages.is_empty() {
            warn!("No pages found in the Notion database. Ingestion finished early.");
            return Ok(IngestionResult {
                documents_added: 0,
                source: db_id,
                document_ids: vec![],
            });
        }

        // 3. Define a unique table name.
        let table_name = format!(
            "notion_{:x}",
            md5::compute(format!("{db_id}::{data_source_id}"))
        );

        // 4. Process pages and store in the database.
        let mut conn = self.db.connect()?;
        process_and_store_pages(&mut conn, &table_name, pages).await?;

        let total_rows: usize = conn
            .query(&format!("SELECT COUNT(*) FROM `{table_name}`"), ())
            .await?
            .next()
            .await?
            .map_or(0, |row| row.get::<i64>(0).unwrap_or(0) as usize);

        info!(
            "Successfully ingested {} pages ({} rows after date expansion) into table `{}`",
            pages_count, total_rows, table_name
        );

        Ok(IngestionResult {
            documents_added: total_rows,
            source: db_id,
            document_ids: vec![table_name], // Use table name as the identifier.
        })
    }
}

// --- Helper Functions ---

fn get_base_url() -> String {
    env::var("NOTION_API_BASE_URL_OVERRIDE_FOR_TESTING")
        .unwrap_or_else(|_| "https://api.notion.com".to_string())
}

fn construct_headers(token: &str, version: &str) -> Result<HeaderMap, NotionError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|e| NotionError::ApiError(format!("Invalid token: {e}")))?,
    );
    headers.insert(
        "Notion-Version",
        HeaderValue::from_str(version)
            .map_err(|e| NotionError::ApiError(format!("Invalid version: {e}")))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    Ok(headers)
}

async fn fetch_database_info(
    client: &reqwest::Client,
    headers: &HeaderMap,
    db_id: &str,
) -> Result<DatabaseResponse, NotionError> {
    let base_url = get_base_url();
    let url = format!("{base_url}/v1/databases/{db_id}");
    let response = client.get(&url).headers(headers.clone()).send().await?;

    if !response.status().is_success() {
        let err_text = response.text().await.unwrap_or_default();
        return Err(NotionError::ApiError(format!(
            "Failed to fetch database info: {err_text}"
        )));
    }
    response
        .json::<DatabaseResponse>()
        .await
        .map_err(|e| e.into())
}

async fn query_all_pages(
    client: &reqwest::Client,
    headers: &HeaderMap,
    data_source_id: &str,
) -> Result<Vec<Page>, NotionError> {
    let mut all_pages = Vec::new();
    let mut next_cursor: Option<String> = None;
    let base_url = get_base_url();
    let url = format!("{base_url}/v1/data_sources/{data_source_id}/query");

    loop {
        let body = json!({ "start_cursor": next_cursor });
        let response = client
            .post(&url)
            .headers(headers.clone())
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let err_text = response.text().await.unwrap_or_default();
            return Err(NotionError::ApiError(format!(
                "Failed to query data source: {err_text}"
            )));
        }

        let mut query_response = response.json::<QueryResponse>().await?;
        all_pages.append(&mut query_response.results);

        if query_response.has_more {
            next_cursor = query_response.next_cursor;
        } else {
            break;
        }
    }

    Ok(all_pages)
}

fn extract_text_from_property(property: &PropertyValue) -> String {
    match property {
        PropertyValue::Title { title } => title
            .iter()
            .map(|t| t.plain_text.clone())
            .collect::<Vec<_>>()
            .join(""),
        PropertyValue::RichText { rich_text } => rich_text
            .iter()
            .map(|t| t.plain_text.clone())
            .collect::<Vec<_>>()
            .join(""),
        _ => "".to_string(),
    }
}

async fn process_and_store_pages(
    conn: &mut Connection,
    table_name: &str,
    pages: Vec<Page>,
) -> Result<(), IngestError> {
    if pages.is_empty() {
        return Ok(());
    }

    let first_page = &pages[0];
    let mut columns: Vec<String> = first_page
        .properties
        .keys()
        .map(|k| format!("`{}`", k.replace('`', "``"))) // Escape column names
        .collect();
    let mut date_range_col: Option<String> = None;

    // Dynamically create columns, identifying the date range column
    for (name, prop) in &first_page.properties {
        if let PropertyValue::Date { .. } = prop {
            if date_range_col.is_some() {
                warn!("Multiple date columns found, only the first one will be used for expansion: {}", name);
            } else {
                date_range_col = Some(name.clone());
                columns.retain(|c| c != &format!("`{name}`")); // Remove original date column
            }
        }
    }

    if date_range_col.is_some() {
        columns.push("`busy_date`".to_string());
        columns.push("`busy_time`".to_string());
    }

    // Create table
    conn.execute(&format!("DROP TABLE IF EXISTS `{table_name}`"), ())
        .await?;
    let create_table_sql = format!(
        "CREATE TABLE `{}` ({})",
        table_name,
        columns
            .iter()
            .map(|c| format!("{c} TEXT"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    conn.execute(&create_table_sql, ()).await?;
    info!("Created table `{}`", table_name);

    // Prepare for insertion
    let placeholders = columns.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let insert_sql = format!(
        "INSERT INTO `{}` ({}) VALUES ({})",
        table_name,
        columns.join(", "),
        placeholders
    );

    let tx = conn.transaction().await?;
    for page in pages {
        let mut base_row_data: HashMap<String, Value> = HashMap::new();
        let mut current_date_prop: Option<PropertyValue> = None;

        for (name, prop) in page.properties {
            if Some(&name) == date_range_col.as_ref() {
                current_date_prop = Some(prop);
            } else {
                base_row_data.insert(
                    format!("`{}`", name.replace('`', "``")),
                    extract_text_from_property(&prop).into(),
                );
            }
        }

        if let (
            Some(_), // We don't need the name, just the fact that it exists
            Some(PropertyValue::Date {
                date: Some(date_val),
            }),
        ) = (date_range_col.as_ref(), current_date_prop)
        {
            // Expand date range
            let start_dt = date_val.start.parse::<DateTime<Utc>>().ok();
            let end_dt = date_val
                .end
                .as_ref()
                .and_then(|s| s.parse::<DateTime<Utc>>().ok());

            if let Some(mut current_dt) = start_dt {
                let end = end_dt.unwrap_or(current_dt);
                while current_dt <= end {
                    let mut row_params: Vec<Value> = Vec::new();
                    for col in &columns {
                        if col == "`busy_date`" {
                            row_params.push(current_dt.format("%Y-%m-%d").to_string().into());
                        } else if col == "`busy_time`" {
                            row_params.push(current_dt.format("%H:%M:%S").to_string().into());
                        } else {
                            row_params.push(base_row_data.get(col).cloned().unwrap_or(Value::Null));
                        }
                    }
                    tx.execute(&insert_sql, params::Params::Positional(row_params))
                        .await?;
                    current_dt += Duration::days(1);
                }
            }
        } else {
            // Regular insert, no date expansion
            let mut row_params: Vec<Value> = Vec::new();
            for col in &columns {
                row_params.push(base_row_data.get(col).cloned().unwrap_or(Value::Null));
            }
            tx.execute(&insert_sql, params::Params::Positional(row_params))
                .await?;
        }
    }
    tx.commit().await?;

    Ok(())
}
