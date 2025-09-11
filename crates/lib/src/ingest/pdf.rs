//! # PDF Ingestion Pipeline
//!
//! This module orchestrates the ingestion of PDF documents into the knowledge base.
//! It supports two main strategies:
//! 1.  **Local Extraction**: Extracts text directly from the PDF using a Rust crate and then
//!     uses a configured LLM to refine it into structured Markdown.
//! 2.  **Gemini Extraction**: Uploads the PDF to Google's API and uses the Gemini model
//!     to perform multimodal extraction and refinement in one step.

#[cfg(feature = "pdf")]
use crate::{
    ingest::knowledge::{
        distill_and_augment, extract_and_store_metadata, store_structured_knowledge, KnowledgeError,
    },
    prompts::pdf::PDF_REFINEMENT_SYSTEM_PROMPT,
    providers::ai::AiProvider,
};
use md5;
use pdf::file::FileOptions;
use serde::de::Error as _; // For the `custom` method on serde_json::Error
use tracing::{info, instrument, warn};
use turso::{params, Database};
use uuid::Uuid;

// --- Data Structures ---

/// Defines the strategy for extracting content from a PDF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfSyncExtractor {
    /// Extract text locally and refine with a generic LLM.
    Local,
    /// Use the Gemini API for multimodal extraction and refinement.
    Gemini,
}

/// Defines the prompts for the PDF ingestion pipeline.
#[derive(Debug)]
pub struct PdfIngestionPrompts<'a> {
    pub distillation_system_prompt: &'a str,
    pub distillation_user_prompt_template: &'a str,
    pub augmentation_system_prompt: &'a str,
    pub metadata_extraction_system_prompt: &'a str,
}

// --- Pipeline Orchestration ---

/// Orchestrates the full ingestion pipeline for a PDF file's content.
///
/// # Arguments
/// * `db`: A reference to the Turso database.
/// * `ai_provider`: A reference to the configured AI provider.
/// * `pdf_data`: The raw byte content of the PDF file.
/// * `source_identifier`: A unique identifier for the source, like the original filename.
/// * `extractor`: The chosen strategy for extracting and refining the content.
///
/// # Returns
/// The number of new Q&A pairs successfully ingested into the knowledge base.
#[instrument(skip(db, ai_provider, pdf_data))]
pub async fn run_pdf_ingestion_pipeline(
    db: &Database,
    ai_provider: &dyn AiProvider,
    pdf_data: Vec<u8>,
    source_identifier: &str,
    owner_id: Option<&str>,
    extractor: PdfSyncExtractor,
    prompts: PdfIngestionPrompts<'_>,
) -> Result<usize, KnowledgeError> {
    info!(
        "Starting PDF ingestion pipeline for '{}' using the '{:?}' extractor.",
        source_identifier, extractor
    );

    let content_hash = format!("{:x}", md5::compute(&pdf_data));

    // --- Stage 1 & 2: Extraction and Refinement ---
    let refined_markdown = match extractor {
        PdfSyncExtractor::Local => {
            // 1. Extract raw text from all pages.
            let raw_text = extract_text_from_pdf(&pdf_data).await?;

            // 2. Refine the raw text using a generic LLM call.
            ai_provider
                .generate(PDF_REFINEMENT_SYSTEM_PROMPT, &raw_text)
                .await?
        }
        PdfSyncExtractor::Gemini => {
            return Err(KnowledgeError::Llm(crate::errors::PromptError::AiApi(
                "Gemini PDF extractor is not yet implemented.".to_string(),
            )));
        }
    };

    // --- Stage 3: Store Refined Content as a Document ---
    let conn = db.connect()?;
    let document_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, source_identifier.as_bytes()).to_string();
    let title: String = refined_markdown.chars().take(80).collect();

    // Use INSERT ... ON CONFLICT to either create a new document or update
    // the content if the PDF is ingested again.
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
    info!(
        "Stored refined PDF content in documents table for source: {}",
        source_identifier
    );

    // --- Stage 4: Distill, Augment, and Extract Metadata (Concurrently) ---
    let ingested_document = crate::ingest::knowledge::IngestedDocument {
        id: document_id.clone(),
        source_url: source_identifier.to_string(),
        content: refined_markdown,
        content_hash,
    };

    let (faq_result, metadata_result) = tokio::join!(
        distill_and_augment(
            ai_provider,
            &ingested_document,
            prompts.distillation_system_prompt,
            prompts.distillation_user_prompt_template,
            prompts.augmentation_system_prompt,
        ),
        extract_and_store_metadata(
            db,
            ai_provider,
            &document_id,
            owner_id,
            &ingested_document.content,
            prompts.metadata_extraction_system_prompt,
        )
    );

    let faq_items = faq_result?;
    metadata_result?; // Propagate metadata errors

    // --- Stage 5: Store Structured Knowledge (Q&A) ---
    store_structured_knowledge(db, &document_id, owner_id, faq_items).await
}

// --- Helper Functions ---

/// Extracts text from all pages of a PDF.
/// This function is designed to be run in a blocking-safe context as PDF parsing is CPU-intensive.
async fn extract_text_from_pdf(pdf_data: &[u8]) -> Result<String, KnowledgeError> {
    info!("Extracting text from PDF...");
    let data = pdf_data.to_vec();

    let text_result = tokio::task::spawn_blocking(move || -> Result<String, KnowledgeError> {
        let file = FileOptions::cached()
            .load(&data[..]) // Pass as a slice to satisfy trait bounds
            .map_err(|e| KnowledgeError::Parse(serde_json::Error::custom(e.to_string())))?;

        let resolver = file.resolver();
        let mut full_text = String::new();

        for page_num in 0..file.num_pages() {
            let page = file
                .get_page(page_num)
                .map_err(|e| KnowledgeError::Parse(serde_json::Error::custom(e.to_string())))?;

            if let Some(content) = &page.contents {
                let operations = content
                    .operations(&resolver)
                    .map_err(|e| KnowledgeError::Parse(serde_json::Error::custom(e.to_string())))?;
                for op in operations.iter() {
                    match op {
                        pdf::content::Op::TextDraw { text } => {
                            full_text.push_str(&text.to_string_lossy());
                        }
                        pdf::content::Op::TextDrawAdjusted { array } => {
                            for item in array.iter() {
                                if let pdf::content::TextDrawAdjusted::Text(text) = item {
                                    full_text.push_str(&text.to_string_lossy());
                                }
                            }
                        }
                        _ => {}
                    }
                }
                full_text.push_str("\n\n"); // Add a separator between pages
            } else {
                warn!("Page {} has no content stream.", page_num);
            }
        }
        Ok(full_text)
    })
    .await;

    // Handle the result of the spawned task, converting JoinError and the inner Result into our KnowledgeError.
    let text: String = text_result.map_err(|e| {
        KnowledgeError::Internal(anyhow::anyhow!("Tokio join error during PDF parsing: {e}"))
    })??;

    info!(
        "Successfully extracted text from PDF. Total length: {} characters.",
        text.len()
    );
    Ok(text)
}
