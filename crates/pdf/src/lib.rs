//! # anyrag-pdf: PDF Ingestion Plugin
//!
//! This crate provides the ingestion logic for PDF documents, acting as a plugin
//! for the `anyrag` ecosystem. It implements the `Ingestor` trait from `anyrag-lib`.

use anyrag::{
    ingest::{IngestError, IngestionResult, Ingestor},
    providers::ai::AiProvider,
    PromptError,
};
use anyrag_web::{extract_and_store_metadata, restructure_with_llm, IngestionPrompts};
use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use pdf::file::FileOptions;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, instrument, warn};
use turso::{params, Database};
use uuid::Uuid;

// --- Error Definitions ---

#[derive(Error, Debug)]
pub enum PdfIngestError {
    #[error("Database error: {0}")]
    Database(#[from] turso::Error),
    #[error("LLM processing failed: {0}")]
    Llm(#[from] PromptError),
    #[error("Failed to parse PDF content: {0}")]
    PdfParse(String),
    #[error("Failed to decode Base64 PDF data: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("An internal error occurred: {0}")]
    Internal(#[from] anyhow::Error),
    #[error("Web ingestor helper failed: {0}")]
    WebHelper(#[from] anyrag_web::WebIngestError),
}

impl From<PdfIngestError> for IngestError {
    fn from(err: PdfIngestError) -> Self {
        match err {
            PdfIngestError::Database(e) => IngestError::Database(e),
            PdfIngestError::PdfParse(s) => IngestError::Parse(s),
            _ => IngestError::Internal(anyhow::anyhow!(err.to_string())),
        }
    }
}

// --- Data Structures ---

#[derive(Debug, Deserialize, Serialize, Default, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PdfExtractor {
    #[default]
    Local,
    Gemini,
}

#[derive(Deserialize)]
struct IngestSource<'a> {
    source_identifier: &'a str,
    pdf_data_base64: &'a str,
    #[serde(default)]
    extractor: PdfExtractor,
}

// --- Core Pipeline Logic ---

/// Extracts text from all pages of a PDF synchronously.
fn extract_text_from_pdf(pdf_data: &[u8]) -> Result<String, PdfIngestError> {
    let file = FileOptions::cached()
        .load(pdf_data)
        .map_err(|e| PdfIngestError::PdfParse(e.to_string()))?;
    let resolver = file.resolver();
    let mut full_text = String::new();

    for page_num in 0..file.num_pages() {
        let page = file
            .get_page(page_num)
            .map_err(|e| PdfIngestError::PdfParse(e.to_string()))?;
        if let Some(content) = &page.contents {
            let operations = content
                .operations(&resolver)
                .map_err(|e| PdfIngestError::PdfParse(e.to_string()))?;
            for op in operations.iter() {
                if let pdf::content::Op::TextDraw { text } = op {
                    full_text.push_str(&text.to_string_lossy());
                }
            }
        }
    }
    Ok(full_text)
}

#[instrument(skip(db, ai_provider, pdf_data))]
async fn run_pdf_ingestion_pipeline(
    db: &Database,
    ai_provider: &dyn AiProvider,
    pdf_data: Vec<u8>,
    source_identifier: &str,
    owner_id: Option<&str>,
    extractor: PdfExtractor,
    prompts: IngestionPrompts<'_>,
) -> Result<usize, PdfIngestError> {
    info!(
        "Starting PDF ingestion pipeline for '{}' using '{:?}' extractor.",
        source_identifier, extractor
    );

    let refined_markdown = match extractor {
        PdfExtractor::Local => {
            extract_text_from_pdf(&pdf_data)?
            // For now, we don't have a separate PDF refinement prompt.
            // We will just use the raw text and let the restructuring handle it.
            // In the future, a refinement step could be added here.
        }
        PdfExtractor::Gemini => {
            return Err(PdfIngestError::Internal(anyhow::anyhow!(
                "Gemini PDF extractor is not yet implemented."
            )));
        }
    };

    if refined_markdown.trim().is_empty() {
        warn!(
            "PDF processing for '{}' resulted in empty content. Aborting.",
            source_identifier
        );
        return Ok(0);
    }

    let conn = db.connect()?;
    let document_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, source_identifier.as_bytes()).to_string();
    let title: String = refined_markdown.chars().take(80).collect();

    conn.execute(
        "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(source_url) DO UPDATE SET content=excluded.content, title=excluded.title",
        params![
            document_id.clone(),
            owner_id,
            source_identifier,
            title,
            refined_markdown.clone()
        ],
    )
    .await?;

    let structured_yaml = restructure_with_llm(
        ai_provider,
        &refined_markdown,
        prompts.restructuring_system_prompt,
    )
    .await?;

    if structured_yaml.trim().is_empty() {
        warn!(
            "LLM restructuring of PDF content for '{}' resulted in empty YAML.",
            source_identifier
        );
        return Ok(0);
    }

    conn.execute(
        "UPDATE documents SET content = ? WHERE id = ?",
        params![structured_yaml.clone(), document_id.clone()],
    )
    .await?;

    extract_and_store_metadata(
        &conn,
        ai_provider,
        &document_id,
        owner_id,
        &structured_yaml,
        prompts.metadata_extraction_system_prompt,
    )
    .await?;

    Ok(1)
}

// --- Ingestor Implementation ---

/// The Ingestor implementation for PDF documents.
pub struct PdfIngestor<'a> {
    db: &'a Database,
    ai_provider: &'a dyn AiProvider,
    prompts: IngestionPrompts<'a>,
}

impl<'a> PdfIngestor<'a> {
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
impl<'a> Ingestor for PdfIngestor<'a> {
    async fn ingest(
        &self,
        source: &str,
        owner_id: Option<&str>,
    ) -> Result<IngestionResult, IngestError> {
        let ingest_source: IngestSource = serde_json::from_str(source)
            .map_err(|e| IngestError::Parse(format!("Invalid source JSON for PDF ingest: {e}")))?;

        let pdf_data = general_purpose::STANDARD
            .decode(ingest_source.pdf_data_base64)
            .map_err(PdfIngestError::from)?;

        let documents_added = run_pdf_ingestion_pipeline(
            self.db,
            self.ai_provider,
            pdf_data,
            ingest_source.source_identifier,
            owner_id,
            ingest_source.extractor,
            self.prompts,
        )
        .await?;

        Ok(IngestionResult {
            source: ingest_source.source_identifier.to_string(),
            documents_added,
            ..Default::default()
        })
    }
}
