//! # Google Sheets FAQ Ingestion Logic
//!
//! This module provides functionality for ingesting Q&A pairs directly
//! from a public Google Sheet into the `faq_kb` knowledge base table.

use crate::ingest::{
    knowledge::{create_kb_tables_if_not_exists, store_structured_knowledge, FaqItem},
    shared::{construct_export_url_and_table_name, download_csv, SheetError},
};
use md5;
use thiserror::Error;
use tracing::{info, warn};
use turso::Database;

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
    #[error(transparent)]
    Knowledge(#[from] crate::ingest::KnowledgeError),
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
    gid: Option<&str>,
    skip_header: bool,
) -> Result<usize, IngestSheetFaqError> {
    info!("Starting FAQ ingestion from Google Sheet URL: {sheet_url}");
    let conn = db.connect()?;
    create_kb_tables_if_not_exists(&conn).await?;

    let (export_url, _) = construct_export_url_and_table_name(sheet_url, gid)?;

    let csv_data = download_csv(&export_url).await?;
    let content_hash = format!("{:x}", md5::compute(csv_data.as_bytes()));

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(skip_header)
        .from_reader(csv_data.as_bytes());

    let (question_idx, answer_idx, start_at_idx, end_at_idx) = if skip_header {
        let headers = reader.headers()?.clone();
        let find_idx = |names: &[&str]| {
            headers.iter().position(|h| {
                let h_trimmed = h.trim();
                names
                    .iter()
                    .any(|name| h_trimmed.eq_ignore_ascii_case(name))
            })
        };

        let question_idx = find_idx(&["question", "questions"]).ok_or_else(|| {
            IngestSheetFaqError::Process(
                "Missing required header: 'question' or 'questions'".to_string(),
            )
        })?;
        let answer_idx = find_idx(&["answer", "answers"]).ok_or_else(|| {
            IngestSheetFaqError::Process(
                "Missing required header: 'answer' or 'answers'".to_string(),
            )
        })?;

        (
            question_idx,
            answer_idx,
            find_idx(&["start_at", "start_date"]),
            find_idx(&["end_at", "end_date"]),
        )
    } else {
        // If there are no headers, assume columns 0 and 1 are Q&A, and 2 and 3 are dates.
        (0, 1, Some(2), Some(3))
    };

    let mut faq_items = Vec::new();
    for result in reader.records() {
        let record = result?;
        let question = record.get(question_idx).unwrap_or("").trim().to_string();
        let mut answer = record.get(answer_idx).unwrap_or("").trim().to_string();

        if question.is_empty() || answer.is_empty() {
            warn!("Skipping row due to empty question or answer.");
            continue;
        }

        let start_at = start_at_idx
            .and_then(|idx| record.get(idx))
            .unwrap_or("")
            .trim();
        let end_at = end_at_idx
            .and_then(|idx| record.get(idx))
            .unwrap_or("")
            .trim();

        if !start_at.is_empty() && !end_at.is_empty() {
            answer = format!("{answer} (effective from {start_at} to {end_at})");
        }

        faq_items.push(FaqItem {
            question,
            answer,
            is_explicit: true,
        });
    }

    if faq_items.is_empty() {
        info!("No valid FAQ items found in the sheet to ingest.");
        return Ok(0);
    }

    info!("Found {} FAQ items to ingest.", faq_items.len());

    let stored_count = store_structured_knowledge(db, sheet_url, &content_hash, faq_items).await?;

    Ok(stored_count)
}
