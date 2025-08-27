//! # Shared Google Sheets Logic
//!
//! This module centralizes common functionality for interacting with Google Sheets,
//! such as URL parsing and data downloading, to be reused by different ingestion modules.

use regex::Regex;
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug, Clone)]
pub enum SheetError {
    #[error("Invalid Google Sheet URL: {0}")]
    InvalidUrl(String),
    #[error("Failed to fetch sheet: {0}")]
    Fetch(String),
}

impl From<reqwest::Error> for SheetError {
    fn from(err: reqwest::Error) -> Self {
        SheetError::Fetch(err.to_string())
    }
}

/// Transforms a Google Sheet URL into a CSV export URL and a sanitized table name.
///
/// This function can optionally include a specific `gid` to target a particular
/// tab within the spreadsheet.
pub fn construct_export_url_and_table_name(
    url_str: &str,
    gid: Option<&str>,
) -> Result<(String, String), SheetError> {
    let parsed_url =
        reqwest::Url::parse(url_str).map_err(|e| SheetError::InvalidUrl(format!("{e}")))?;

    let re = Regex::new(r"/spreadsheets/d/([a-zA-Z0-9-_]+)")
        .map_err(|e| SheetError::InvalidUrl(format!("Regex compilation failed: {e}")))?;
    let caps = re.captures(parsed_url.path()).ok_or_else(|| {
        SheetError::InvalidUrl("Could not find sheet ID in URL path.".to_string())
    })?;

    let spreadsheets_id = caps
        .get(1)
        .map(|m| m.as_str())
        .ok_or_else(|| SheetError::InvalidUrl("Sheet ID capture group is missing.".to_string()))?;

    let base_url = match parsed_url.host_str() {
        Some("127.0.0.1") | Some("localhost") => {
            format!("{}://{}", parsed_url.scheme(), parsed_url.authority())
        }
        _ => "https://docs.google.com".to_string(),
    };
    let mut export_url = format!("{base_url}/spreadsheets/d/{spreadsheets_id}/export?format=csv");

    if let Some(gid_val) = gid {
        if !gid_val.is_empty() {
            export_url.push_str(&format!("&gid={gid_val}"));
        }
    }

    let table_name = format!("spreadsheets_{}", spreadsheets_id.replace('-', "_"));

    Ok((export_url, table_name))
}

/// Downloads the content of a Google Sheet as a CSV string.
pub async fn download_csv(export_url: &str) -> Result<String, SheetError> {
    info!("Fetching Google Sheet CSV from: {export_url}");
    let response = reqwest::get(export_url).await?;
    if !response.status().is_success() {
        return Err(SheetError::Fetch(format!(
            "Request failed with status: {}",
            response.status()
        )));
    }
    response.text().await.map_err(SheetError::from)
}
