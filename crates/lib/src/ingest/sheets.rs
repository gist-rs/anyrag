//! # Google Sheets Ingestion Logic
//!
//! This module provides the functionality for ingesting data from a public
//! Google Sheet and storing it in a local SQLite database.

use regex::Regex;
use thiserror::Error;
use tracing::{info, warn};
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
///
/// This function is public to allow the server to determine the target table name
/// before calling the ingestion logic. It handles both real Google Sheet URLs and
/// local test URLs from `httpmock`.
pub fn sheet_url_to_export_url_and_table_name(
    url_str: &str,
) -> Result<(String, String), IngestSheetError> {
    // Use the `url` crate's parser, which `reqwest` re-exports.
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

    // Determine the base URL for the export link.
    // If it's a local test server, use its address. Otherwise, use the real Google Sheets URL.
    let base_url = match parsed_url.host_str() {
        Some("127.0.0.1") | Some("localhost") => {
            format!("{}://{}", parsed_url.scheme(), parsed_url.authority())
        }
        _ => "https://docs.google.com".to_string(),
    };

    let export_url = format!("{base_url}/spreadsheets/d/{spreadsheets_id}/export?format=csv");
    // Sanitize the spreadsheets_id to be a valid table name prefix.
    let table_name = format!("spreadsheets_{}", spreadsheets_id.replace('-', "_"));

    Ok((export_url, table_name))
}

/// Fetches a Google Sheet, parses it as CSV, and ingests it into a new SQLite table.
///
/// This function no longer checks for the table's existence, assuming the caller has
/// already performed this check. It is designed to be called only when ingestion is needed.
///
/// # Arguments
///
/// * `db`: A shared reference to the Turso database instance.
/// * `export_url`: The direct CSV export URL for the Google Sheet.
/// * `table_name`: The name of the table to create.
///
/// # Returns
///
/// The number of rows inserted.
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

    let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
    let headers = reader.headers()?.clone();
    if headers.is_empty() {
        return Err(IngestSheetError::NoData);
    }

    // Sanitize headers to be valid column names (alphanumeric and underscores).
    let sanitized_headers: Vec<String> = headers
        .iter()
        .map(|h| {
            h.trim()
                .to_lowercase()
                .replace(' ', "_")
                .replace(|c: char| !c.is_alphanumeric() && c != '_', "")
        })
        .collect();

    create_table_from_headers(&conn, table_name, &sanitized_headers).await?;

    conn.execute("BEGIN TRANSACTION", ()).await?;
    let mut insert_count = 0;

    // Prepare the INSERT statement dynamically.
    let columns = sanitized_headers.join(", ");
    let values_placeholders = (0..sanitized_headers.len())
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");
    let insert_sql = format!("INSERT INTO {table_name} ({columns}) VALUES ({values_placeholders})");

    let mut stmt = conn.prepare(&insert_sql).await?;

    for result in reader.records() {
        let record = result?;
        let params: Vec<TursoValue> = record
            .iter()
            .map(|field| TursoValue::Text(field.to_string()))
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

/// Creates a new table based on CSV headers.
async fn create_table_from_headers(
    conn: &Connection,
    table_name: &str,
    headers: &[String],
) -> Result<(), turso::Error> {
    // We assume all columns are TEXT for simplicity and robustness.
    let columns_def = headers
        .iter()
        .map(|h| format!("\"{h}\" TEXT"))
        .collect::<Vec<_>>()
        .join(", ");

    let create_sql = format!("CREATE TABLE IF NOT EXISTS {table_name} ({columns_def});");
    info!("Executing CREATE TABLE statement: {create_sql}");
    conn.execute(&create_sql, ()).await?;
    Ok(())
}
