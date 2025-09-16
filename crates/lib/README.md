# `anyrag` Library

This crate provides the core logic for a comprehensive natural language data interaction and Retrieval-Augmented Generation (RAG) platform. It has been refactored to focus on a new, structured-data approach that significantly improves retrieval accuracy and context awareness.

## Core Concept: The YAML-Based Pipeline

The central philosophy of this library is to move away from disconnected, fragmented data (like individual FAQs) and towards a single, structured "source of truth" for each ingested document. We use an LLM to intelligently restructure messy source content into a clean, hierarchical YAML format. This YAML document then becomes the foundation for all subsequent RAG and fine-tuning tasks.

## Features

*   **Knowledge Base Pipeline:** A complete "virtuous cycle" for RAG:
    *   **Multi-Source Ingestion:** Ingests and processes content from web URLs, PDFs, Google Sheets, RSS feeds, and raw text.
    *   **LLM-Powered Restructuring:** Uses an LLM to intelligently reformat messy source content (like HTML converted to Markdown) into a structured YAML format, preserving the original document's semantic hierarchy.
    *   **Store & Embed:** Saves the final YAML as a single entry in a local SQLite database and generates a vector embedding for the entire document for semantic search.

*   **Advanced Retrieval-Augmented Generation (RAG):**
    *   **RAG-on-YAML:** Synthesizes answers by first retrieving relevant parent documents via hybrid search, then parsing their YAML content on-the-fly and using the structured `sections` as context-rich "chunks."
    *   **Temporal Reasoning:** Understands time-sensitive queries like "what is the newest..." by filtering results based on date properties.

*   **Fine-Tuning Export:** Exports a clean, high-quality dataset for fine-tuning by simply parsing the structured YAML stored in the database.
*   **Pluggable Providers:** Supports different AI and storage providers (e.g., Gemini, local models, SQLite).
*   **Identity & Ownership (`core-access` feature):** Provides a flexible user and ownership model to ensure clear data provenance and secure, ownership-aware search.

## The New RAG-on-YAML Pipeline

The system uses a multi-stage process to deliver a precise answer:

1.  **Query Analysis (LLM Call #1):** The user's query is analyzed to extract key **entities** and **keyphrases**.
2.  **Hybrid Candidate Retrieval:** A combination of metadata, keyword, and vector searches are run to find the most relevant *parent documents*. The vector search uses an **Embedding Model** to convert the user's query into a vector.
3.  **Reciprocal Rank Fusion (RRF):** The results from all search methods are intelligently combined and re-ranked using the RRF algorithm.
4.  **YAML Parsing & Contextual Chunking:** The system parses the structured YAML content of the top-ranked parent documents. It treats each `section` within the YAML as a single, context-rich "chunk."
5.  **Answer Synthesis (LLM Call #2):** The final, highly-relevant chunks are passed to the synthesis LLM, which generates a coherent, accurate answer based *only* on the provided, structured information.

## Basic Usage

This crate is a library. Its components are orchestrated by binaries like `anyrag-server`. Here is a high-level example of how you might use the ingestion pipeline directly:

```rust
use anyrag::{
    ingest::{run_ingestion_pipeline, knowledge::{IngestionPrompts, WebIngestStrategy}},
    providers::{ai::local::LocalAiProvider, db::sqlite::SqliteProvider},
};

async fn ingest_a_url() {
    let db_provider = SqliteProvider::new(":memory:").await.unwrap();
    db_provider.initialize_schema().await.unwrap();
    let ai_provider = LocalAiProvider::new("http://localhost:1234/v1/chat/completions".to_string(), None, None).unwrap();

    let prompts = IngestionPrompts {
        restructuring_system_prompt: "You are an expert document analyst...",
        metadata_extraction_system_prompt: "You are an expert metadata extractor...",
    };

    let _ingested_count = run_ingestion_pipeline(
        &db_provider.db,
        &ai_provider,
        "https://example.com",
        None,
        prompts,
        WebIngestStrategy::RawHtml,
    ).await.unwrap();
}
```

## Configuration

This library does not directly read configuration files or environment variables. It is designed to be configured by the consuming application (e.g., `anyrag-server`), which passes the necessary providers, prompts, and settings to the library's functions.

## Running Tests

You can run the tests for this specific crate from the workspace root:

```sh
cargo test -p anyrag
```
