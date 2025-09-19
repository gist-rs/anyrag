//! # Google Sheets FAQ Ingestion Logic
//!
//! This module provides functionality for ingesting Q&A pairs directly
//! from a public Google Sheet into the `faq_kb` knowledge base table.

use crate::ingest::shared::{construct_export_url_and_table_name, download_csv, SheetError};
use thiserror::Error;
use tracing::{info, warn};
use turso::{params, Database};
use uuid::Uuid;

/// Custom error types for the Google Sheet FAQ ingestion process.
#[derive(Error, Debug)]
pub enum IngestSheetFaqError {
    #[error("Database error: {0}")]
    Database(#[from] turso::Error),
    #[error("Failed to fetch or parse sheet: {0}")]
    Sheet(#[from] SheetError),
    #[error("Failed to parse CSV from sheet: {0}")]
    Parse(#[from] csv::Error),
    #[error("Sheet processing error: {0}")]
    Process(String),
}

/// Fetches a Google Sheet, parses it as a Q&A list, and ingests it into the `faq_kb` table.
///
/// # Arguments
/// * `db`: A reference to the Turso database.
/// * `sheet_url`: The public URL of the Google Sheet.
/// * `gid`: The specific tab/sheet ID to target.
/// * `skip_header`: Whether to skip the first row (typically headers).
///
/// # Returns
/// The number of new FAQ pairs successfully ingested.
pub async fn ingest_faq_from_google_sheet(
    db: &Database,
    sheet_url: &str,
    owner_id: Option<&str>,
    gid: Option<&str>,
    _skip_header: bool,
) -> Result<usize, IngestSheetFaqError> {
    info!("Starting FAQ ingestion from Google Sheet URL: {sheet_url}");
    let conn = db.connect()?;

    let (export_url, _) = construct_export_url_and_table_name(sheet_url, gid)?;
    let csv_data = download_csv(&export_url).await?;

    // Create a single parent document for this sheet
    let document_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, sheet_url.as_bytes()).to_string();
    let document_title = format!("FAQs from sheet: {sheet_url}");

    // Use INSERT ... ON CONFLICT DO NOTHING to avoid errors on re-ingestion.
    // We only create the parent document if it doesn't exist. The FAQs themselves
    // will be overwritten by the `store_faq_items` logic.
    conn.execute(
        "INSERT INTO documents (id, owner_id, source_url, title, content)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(source_url) DO UPDATE SET
         title = excluded.title,
         content = excluded.content",
        params![
            document_id.clone(),
            owner_id,
            sheet_url,
            document_title,
            csv_data.clone() // Store the raw CSV as the document content
        ],
    )
    .await?;

    // TODO: Refactor this to use the new LLM-based YAML restructuring.
    // For now, this function only creates the parent document.
    warn!("DEPRECATED: `ingest_faq_from_google_sheet` no longer stores individual FAQs. Refactor needed.");
    Ok(0)
}
