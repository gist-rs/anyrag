//! # PDF Ingestion Pipeline
//!
//! This module orchestrates the ingestion of PDF documents into the knowledge base.
//! It supports two main strategies:
//! 1.  **Local Extraction**: Extracts text directly from the PDF using a Rust crate and then
//!     uses a configured LLM to refine it into structured Markdown.
//! 2.  **Gemini Extraction**: Uploads the PDF to Google's API and uses the Gemini model
//!     to perform multimodal extraction and refinement in one step.

use crate::{
    ingest::knowledge::{distill_and_augment, store_structured_knowledge, KnowledgeError},
    providers::ai::AiProvider,
};
use md5;
use pdf::file::FileOptions;
use serde::de::Error as _; // For the `custom` method on serde_json::Error
use tracing::{info, instrument};
use turso::{params, Connection, Database};

// --- Prompts ---

/// The system prompt used to instruct the LLM to refine extracted text into structured Markdown.
pub const PDF_REFINEMENT_SYSTEM_PROMPT: &str = r#"You are an expert technical analyst. Your task is to process the content of the provided document text and reformat it into a clean, well-structured Markdown document. Extract all key information, including topics, sub-topics, questions, and important data points. Use headings (#, ##), lists (*), and bold text (**text**) to organize the content logically. Do not summarize or omit details; the goal is to create a comprehensive and machine-readable version of the original content that preserves all facts."#;

// --- Data Structures ---

/// Defines the strategy for extracting content from a PDF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfSyncExtractor {
    /// Extract text locally and refine with a generic LLM.
    Local,
    /// Use the Gemini API for multimodal extraction and refinement.
    Gemini,
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
    extractor: PdfSyncExtractor,
) -> Result<usize, KnowledgeError> {
    info!(
        "Starting PDF ingestion pipeline for '{}' using the '{:?}' extractor.",
        source_identifier, extractor
    );

    let content_hash = format!("{:x}", md5::compute(&pdf_data));

    // TODO: Check if content_hash already exists in `refined_content` to avoid reprocessing.

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
            // The Gemini-specific logic will be implemented here.
            // For now, it returns an error as per the plan.
            return Err(KnowledgeError::Llm(crate::errors::PromptError::AiApi(
                "Gemini PDF extractor is not yet implemented.".to_string(),
            )));
        }
    };

    // --- Stage 3: Store Refined Content ---
    let conn = db.connect()?;
    store_refined_content(&conn, source_identifier, &refined_markdown, &content_hash).await?;

    // --- Stage 4: Distill & Augment ---
    let raw_content_for_distill = crate::ingest::knowledge::RawContent {
        url: source_identifier.to_string(), // Use source_identifier as the unique key
        markdown_content: refined_markdown,
        content_hash: content_hash.clone(),
    };
    let faq_items = distill_and_augment(ai_provider, &raw_content_for_distill).await?;

    // --- Stage 5: Store Structured Knowledge (Q&A) ---
    store_structured_knowledge(db, source_identifier, &content_hash, faq_items).await
}

// --- Helper Functions ---

/// Extracts text from all pages of a PDF.
///
/// This function is designed to be run in a blocking-safe context as PDF parsing is CPU-intensive.
async fn extract_text_from_pdf(pdf_data: &[u8]) -> Result<String, KnowledgeError> {
    info!("Extracting text from PDF...");
    // The `pdf` crate requires the data to be owned, so we clone it for the blocking task.
    let data = pdf_data.to_vec();

    // Spawn a blocking task to avoid stalling the async runtime with CPU-intensive PDF parsing.
    let text_result = tokio::task::spawn_blocking(move || {
        let file = FileOptions::cached()
            .load(&data)
            .map_err(|e| KnowledgeError::Parse(serde_json::Error::custom(e.to_string())))?;
        let all_pages = 0..file.num_pages();
        pdf::text::extract_text(&file, all_pages)
            .map_err(|e| KnowledgeError::Parse(serde_json::Error::custom(e.to_string())))
    })
    .await;

    // Handle the result of the spawned task, converting JoinError and the inner Result into our KnowledgeError.
    let text = text_result.map_err(|e| {
        KnowledgeError::Internal(anyhow::anyhow!("Tokio join error during PDF parsing: {e}"))
    })??;

    info!(
        "Successfully extracted text from PDF. Total length: {} characters.",
        text.len()
    );
    Ok(text)
}

/// Stores the LLM-refined Markdown content in the database.
async fn store_refined_content(
    conn: &Connection,
    source_identifier: &str,
    refined_markdown: &str,
    raw_content_hash: &str,
) -> Result<(), KnowledgeError> {
    info!("Storing refined markdown for source: {}", source_identifier);
    conn.execute(
        "INSERT INTO refined_content (source_identifier, refined_markdown, raw_content_hash) VALUES (?, ?, ?)",
        params![source_identifier, refined_markdown, raw_content_hash],
    ).await?;
    Ok(())
}
