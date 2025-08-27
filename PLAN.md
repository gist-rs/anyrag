# Plan: PDF Ingestion for Knowledge Base

This document outlines the implementation plan for adding PDF file ingestion capabilities to the `anyrag` project. The goal is to allow users to build a knowledge base from PDF documents, leveraging both local processing and advanced multimodal AI models.

## 1. Core Concepts & Workflow

The feature will introduce two distinct strategies for processing PDFs, both of which feed into the existing knowledge distillation pipeline.

### Key Refinements
- **LLM Refinement:** Instead of simple summarization, which can lead to information loss, all extraction methods will be followed by a mandatory LLM refinement step. This step will use a specialized prompt to convert the raw extracted text into a well-structured, detailed Markdown document. This ensures high-quality, consistent input for the final Q&A generation.
- **Intermediate Data Storage:** The LLM-refined Markdown will be stored in a new, dedicated database table (`refined_content`) for traceability, debugging, and future enhancements (e.g., knowledge graph extraction).

### A. Local Extraction Workflow
This workflow is designed for users who want to process files locally, using their CPU for text extraction and a configured LLM (which could be a local model) for refinement.

1.  **File Upload:** The user uploads a PDF to the server.
2.  **Parallel Text Extraction:** The `anyrag` library uses a Rust crate (`pdf`) to parse the document. It extracts raw text from each page in parallel (defaulting to 2 concurrent pages) to efficiently handle large documents.
3.  **LLM Refinement:** The combined raw text is sent to the configured AI provider with a prompt to "extract key points as Markdown."
4.  **Store Refined Content:** The resulting clean Markdown is saved to the new `refined_content` table.
5.  **Knowledge Distillation:** The clean Markdown is passed to the existing `distill_and_augment` pipeline to generate Q&A pairs.
6.  **Store Q&A:** The final Q&A pairs are stored in the `faq_kb` table.

### B. Gemini (Multimodal) Extraction Workflow
This workflow leverages the advanced capabilities of the Gemini Pro Vision API to process the entire PDF, including images and layout, for a potentially higher-quality extraction.

1.  **File Upload (to Google):** The server receives the PDF and uploads it directly to the Google Cloud file storage API, receiving a file resource name.
2.  **LLM Refinement:** The `anyrag` library makes a `generateContent` request to the Gemini Vision model, referencing the uploaded file and using the same "extract key points as Markdown" prompt.
3.  **Store Refined Content:** The resulting clean Markdown from Gemini is saved to the new `refined_content` table.
4.  **Knowledge Distillation & Storage:** The process merges with the local workflow, feeding the clean Markdown into the `distill_and_augment` pipeline and storing the final Q&A pairs.

## 2. Implementation Phases

The work will be broken down into three distinct phases.

### Phase 1: Core Library Enhancements (`anyrag` crate)

-   [ ] **Dependency:** Add the `pdf` crate to `anyrag/crates/lib/Cargo.toml`.
-   [ ] **Database Schema:** Update `ingest::knowledge::create_kb_tables_if_not_exists` to create the new `refined_content` table.
-   [ ] **PDF Module:** Create a new `ingest::pdf` module.
    -   Implement `extract_text_from_pdf`, which will handle the parallel page processing.
-   [ ] **AI Provider Update:**
    -   Enhance the `AiProvider` trait with a new method for multimodal/file-based content generation.
    -   Implement this new method for `GeminiProvider`, handling the two-step upload and `generateContent` call.
    -   Update `LocalAiProvider` to return a `Not Supported` error for this new method.
-   [ ] **Knowledge Pipeline Update:**
    -   Create a `store_refined_content` function in `ingest::knowledge`.
    -   Create a new top-level `run_pdf_ingestion_pipeline` function that orchestrates the entire workflow (local vs. Gemini), including the call to `store_refined_content`.

### Phase 2: Server API (`anyrag-server` crate)

-   [ ] **Dependencies:** Add `axum-extra` for `multipart/form-data` handling.
-   [ ] **New Route:** Add a `POST /ingest/file` route to `router.rs`.
-   [ ] **New Handler:**
    -   Implement `ingest_file_handler` in `handlers.rs`.
    -   This handler will process `multipart/form-data` requests, expecting a file part and an optional `extractor` field (`local` or `gemini`).
    -   It will call the `run_pdf_ingestion_pipeline` library function with the file data and selected strategy.
    -   It will return a JSON response summarizing the result.

### Phase 3: Documentation

-   [ ] **Update `anyrag-server/README.md`:**
    -   Add a new section for the `POST /ingest/file` endpoint.
    -   Provide a `curl` example demonstrating how to upload a file and specify the extractor.