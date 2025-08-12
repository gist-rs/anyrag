//! # Google Sheets Ingestion Logic
//!
//! This module provides the functionality for ingesting data from a public
//! Google Sheet and storing it in a local SQLite database. It includes logic
//! to "sniff" column types to create a more descriptive and useful schema.

use chrono::NaiveDateTime;
use regex::Regex;
use thiserror::Error;
use tracing::{debug, info, warn};
use turso::{Connection, Database, Value as TursoValue};

/// Custom error types for the Google Sheet ingestion process.
#[derive(Error, Debug)]
pub enum IngestSheetError {
    #[error("Database error: {0}")]
    Database(#[from] turso::Error),
    #[error("Failed to fetch sheet: {0}")]
    Fetch(#[from] reqwest::Error),
    #[error("Failed to parse CSV from sheet: {0}")]
    Parse(#[from] csv::Error),
    #[error("Invalid Google Sheet URL: {0}")]
    InvalidUrl(String),
    #[error("The sheet has no data to ingest.")]
    NoData,
    #[error("Failed to get database connection: {0}")]
    Connection(String),
}

/// Transforms a Google Sheet URL into a CSV export URL and a sanitized table name.
pub fn sheet_url_to_export_url_and_table_name(
    url_str: &str,
) -> Result<(String, String), IngestSheetError> {
    let parsed_url = reqwest::Url::parse(url_str)
        .map_err(|e| IngestSheetError::InvalidUrl(format!("Failed to parse URL: {e}")))?;

    let re = Regex::new(r"/spreadsheets/d/([a-zA-Z0-9-_]+)")
        .map_err(|e| IngestSheetError::InvalidUrl(format!("Regex compilation failed: {e}")))?;
    let caps = re.captures(parsed_url.path()).ok_or_else(|| {
        IngestSheetError::InvalidUrl("Could not find sheet ID in URL path.".to_string())
    })?;

    let spreadsheets_id = caps.get(1).map(|m| m.as_str()).ok_or_else(|| {
        IngestSheetError::InvalidUrl("Sheet ID capture group is missing.".to_string())
    })?;

    let base_url = match parsed_url.host_str() {
        Some("127.0.0.1") | Some("localhost") => {
            format!("{}://{}", parsed_url.scheme(), parsed_url.authority())
        }
        _ => "https://docs.google.com".to_string(),
    };

    let export_url = format!("{base_url}/spreadsheets/d/{spreadsheets_id}/export?format=csv");
    let table_name = format!("spreadsheets_{}", spreadsheets_id.replace('-', "_"));

    Ok((export_url, table_name))
}

/// Fetches a Google Sheet, parses it as CSV, and ingests it into a new SQLite table.
pub async fn ingest_from_google_sheet_url(
    db: &Database,
    export_url: &str,
    table_name: &str,
) -> Result<usize, IngestSheetError> {
    info!("[ingest_sheet] Attempting to get database connection...");
    let conn = db
        .connect()
        .map_err(|e| IngestSheetError::Connection(e.to_string()))?;
    info!("[ingest_sheet] Successfully got database connection.");

    info!("Fetching Google Sheet from: {export_url}");
    let response = reqwest::get(export_url).await?;
    if !response.status().is_success() {
        return Err(IngestSheetError::Fetch(
            response.error_for_status().unwrap_err(),
        ));
    }
    let csv_data = response.text().await?;
    debug!("[ingest_sheet] Raw CSV data: {}", csv_data);

    let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
    let headers = reader.headers()?.clone();
    if headers.is_empty() {
        return Err(IngestSheetError::NoData);
    }

    // Collect all records to analyze the first row for types
    let records: Vec<csv::StringRecord> = reader.records().collect::<Result<_, _>>()?;
    if records.is_empty() {
        return Err(IngestSheetError::NoData);
    }

    // Sanitize headers for column names
    let sanitized_headers: Vec<String> = headers
        .iter()
        .map(|h| {
            h.trim()
                .to_lowercase()
                .replace(' ', "_")
                .replace(|c: char| !c.is_alphanumeric() && c != '_', "")
        })
        .collect();

    // Sniff column types and get chrono parse formats from the first data row
    let column_parse_info = sniff_column_parse_info(&records[0]);
    // Extract just the DB types for the CREATE TABLE statement
    let column_db_types: Vec<String> = column_parse_info
        .iter()
        .map(|(db_type, _)| db_type.clone())
        .collect();
    create_table_from_headers(&conn, table_name, &sanitized_headers, &column_db_types).await?;

    conn.execute("BEGIN TRANSACTION", ()).await?;
    let mut insert_count = 0;

    let columns = sanitized_headers.join(", ");
    let values_placeholders = (0..sanitized_headers.len())
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");
    let insert_sql = format!("INSERT INTO {table_name} ({columns}) VALUES ({values_placeholders})");
    let mut stmt = conn.prepare(&insert_sql).await?;

    for record in records {
        // Transform the record, standardizing date formats
        let params: Vec<TursoValue> = record
            .iter()
            .zip(column_parse_info.iter())
            .map(|(field, (_, parse_format))| {
                if let Some(fmt) = parse_format {
                    // If a parse format is available, try to parse and reformat.
                    if let Ok(dt) = NaiveDateTime::parse_from_str(field, fmt) {
                        return TursoValue::Text(dt.format("%Y-%m-%d %H:%M:%S").to_string());
                    }
                }
                // Fallback to inserting the original text.
                TursoValue::Text(field.to_string())
            })
            .collect();

        match stmt.execute(params).await {
            Ok(changes) => {
                if changes > 0 {
                    insert_count += 1;
                }
            }
            Err(e) => {
                warn!("Failed to insert row: {e:?}. Rolling back transaction.");
                conn.execute("ROLLBACK", ()).await?;
                return Err(IngestSheetError::Database(e));
            }
        }
    }

    conn.execute("COMMIT", ()).await?;
    info!("Transaction committed. Ingested {insert_count} new rows into '{table_name}'.");

    Ok(insert_count)
}

/// Analyzes the first row of data to infer column types and parsing formats.
/// Returns a tuple of (SQLite_TYPE, OPTIONAL_CHRONO_PARSE_FORMAT).
fn sniff_column_parse_info(
    first_record: &csv::StringRecord,
) -> Vec<(String, Option<&'static str>)> {
    const DATE_FORMATS: [&str; 3] = ["%-m/%-d/%Y %-H:%M:%S", "%Y-%m-%d %H:%M:%S", "%Y-%m-%d"];

    first_record
        .iter()
        .map(|field| {
            if field.parse::<i64>().is_ok() {
                return ("INTEGER".to_string(), None);
            }
            if field.parse::<f64>().is_ok() {
                return ("REAL".to_string(), None);
            }
            for parse_fmt in DATE_FORMATS {
                if NaiveDateTime::parse_from_str(field, parse_fmt).is_ok() {
                    return ("DATETIME".to_string(), Some(parse_fmt));
                }
            }
            ("TEXT".to_string(), None)
        })
        .collect()
}

/// Creates a new table with simple column types.
async fn create_table_from_headers(
    conn: &Connection,
    table_name: &str,
    headers: &[String],
    column_types: &[String],
) -> Result<(), turso::Error> {
    let columns_def = headers
        .iter()
        .zip(column_types.iter())
        .map(|(h, t)| format!("\"{h}\" {t}"))
        .collect::<Vec<_>>()
        .join(", ");

    let create_sql = format!("CREATE TABLE IF NOT EXISTS {table_name} ({columns_def});");
    info!("Executing CREATE TABLE statement: {create_sql}");
    conn.execute(&create_sql, ()).await?;
    Ok(())
}
