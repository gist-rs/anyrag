//! # `anyrag-sheets`: Google Sheets Ingestion Plugin
//!
//! This crate provides the logic for ingesting data from Google Sheets as a self-contained
//! plugin for the `anyrag` ecosystem. It implements the `Ingestor` trait from the
//! core `anyrag` library.

use anyhow::anyhow;
use anyrag::{
    ingest::{
        knowledge::{extract_and_store_metadata, restructure_with_llm},
        traits::{IngestError, IngestionPrompts, IngestionResult, Ingestor},
    },
    providers::ai::AiProvider,
};
use async_trait::async_trait;
use regex::Regex;
use serde::Deserialize;
use thiserror::Error;
use tracing::info;
use turso::Database;
use uuid::Uuid;

// --- Error Definitions ---

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

/// A helper to convert the specific `SheetError` into the generic `anyrag::ingest::IngestError`.
impl From<SheetError> for IngestError {
    fn from(err: SheetError) -> Self {
        match err {
            SheetError::InvalidUrl(msg) => IngestError::SourceNotFound(msg),
            SheetError::Fetch(msg) => IngestError::Fetch(msg),
        }
    }
}

// --- Public Helper Functions ---

/// Transforms a Google Sheet URL into a CSV export URL.
pub fn construct_export_url(url_str: &str, gid: Option<&str>) -> Result<String, SheetError> {
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

    Ok(export_url)
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

// --- Ingestor Implementation ---

/// Defines the structure of the JSON string passed to the `ingest` method.
#[derive(Deserialize)]
struct SheetSource {
    url: String,
    gid: Option<String>,
}

/// The `Ingestor` implementation for Google Sheets.
pub struct SheetsIngestor<'a> {
    db: &'a Database,
    ai_provider: &'a dyn AiProvider,
    prompts: IngestionPrompts<'a>,
}

impl<'a> SheetsIngestor<'a> {
    /// Creates a new `SheetsIngestor`.
    pub fn new(
        db: &'a Database,
        ai_provider: &'a dyn AiProvider,
        prompts: IngestionPrompts<'a>,
    ) -> Self {
        Self {
            db,
            ai_provider,
            prompts,
        }
    }
}

#[async_trait]
impl Ingestor for SheetsIngestor<'_> {
    /// Ingests a Google Sheet using the modern knowledge pipeline.
    ///
    /// The `source` argument is expected to be a JSON string with a `url` key
    /// and an optional `gid` key, for example:
    /// `{"url": "https://docs.google.com/spreadsheets/d/...", "gid": "12345"}`.
    async fn ingest(
        &self,
        source: &str,
        owner_id: Option<&str>,
    ) -> Result<IngestionResult, IngestError> {
        let sheet_source: SheetSource = serde_json::from_str(source)
            .map_err(|e| IngestError::Parse(format!("Failed to parse SheetSource JSON: {e}")))?;

        // --- 1. Download CSV content from Google Sheet ---
        let export_url = construct_export_url(&sheet_source.url, sheet_source.gid.as_deref())?;
        let csv_content = download_csv(&export_url).await?;

        // --- 2. Create or Update Parent Document ---
        let conn = self.db.connect()?;
        let document_id: String;

        if let Some(row) = conn
            .query(
                "SELECT id FROM documents WHERE source_url = ?",
                turso::params![sheet_source.url.clone()],
            )
            .await?
            .next()
            .await?
        {
            document_id = row.get(0)?;
        } else {
            document_id =
                Uuid::new_v5(&Uuid::NAMESPACE_URL, sheet_source.url.as_bytes()).to_string();
            let title = format!("Data from sheet: {}", sheet_source.url);
            conn.execute(
                "INSERT INTO documents (id, owner_id, source_url, title, content)
                 VALUES (?, ?, ?, ?, ?)
                 ON CONFLICT(source_url) DO UPDATE SET
                 title = excluded.title,
                 content = excluded.content",
                turso::params![
                    document_id.clone(),
                    owner_id,
                    sheet_source.url.clone(),
                    title,
                    csv_content.clone() // Store raw CSV initially
                ],
            )
            .await?;
        }

        // --- 3. Restructure CSV to YAML using LLM ---
        let structured_yaml = restructure_with_llm(
            self.ai_provider,
            &csv_content,
            self.prompts.restructuring_system_prompt,
        )
        .await
        .map_err(|e| IngestError::Internal(anyhow!("LLM restructuring failed: {e}")))?;

        // --- 4. Update Document and Extract Metadata ---
        conn.execute(
            "UPDATE documents SET content = ? WHERE id = ?",
            turso::params![structured_yaml.clone(), document_id.clone()],
        )
        .await?;

        extract_and_store_metadata(
            &conn,
            self.ai_provider,
            &document_id,
            owner_id,
            &structured_yaml,
            self.prompts.metadata_extraction_system_prompt,
        )
        .await
        .map_err(|e| IngestError::Internal(anyhow!("Metadata extraction failed: {e}")))?;

        info!(
            "Successfully ingested and processed Google Sheet as document ID: {}",
            document_id
        );

        Ok(IngestionResult {
            documents_added: 1, // The entire sheet is treated as one document.
            source: sheet_source.url,
            document_ids: vec![document_id],
            metadata: None,
        })
    }
}
