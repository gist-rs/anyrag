# `anyrag` Library

This crate provides the core logic for a comprehensive natural language data interaction and RAG platform. Its main functionalities include:
1.  **Natural Language to Data:** Translating prompts into executable queries for data warehouses like Google BigQuery, or dynamically ingesting and querying data from sources like Google Sheets.
2.  **RAG Pipeline:** A complete system for building a self-improving knowledge base from diverse sources (web pages, PDFs, text, and structured sheets) and answering questions using an advanced, multi-stage hybrid search model.

It uses a pluggable AI provider for NLP and integrates with both remote (BigQuery) and local (SQLite) storage backends.

This library is the foundation of the `anyrag` workspace and is used by the `anyrag-server` crate to expose its functionality over a REST API.

## Features

*   **Natural Language to Query:**
    *   Converts plain English prompts into executable SQL queries.
    *   Can dynamically ingest and query Google Sheets when a URL is provided in the prompt.
    *   Automatically injects the current date into the AI's context for handling time-sensitive questions.
*   **Knowledge Base Pipeline:** A complete "virtuous cycle" for RAG:
    *   **Multi-Source Ingestion:** Ingests and processes content from:
        -   Web URLs (fetching and cleaning Markdown).
        -   PDF files (from uploads or direct URLs).
        -   Google Sheets (for structured, time-sensitive FAQs).
        -   RSS feeds (for continuous content updates).
        -   Raw text (with automatic chunking).
    *   **Distill & Augment:** Uses a two-pass LLM process to extract explicit FAQs and generate new ones from unstructured content.
    *   **Store & Embed:** Saves structured Q&A pairs into a local SQLite database and generates vector embeddings for semantic search.
    *   **Export for Fine-tuning:** Generates a dataset in the correct format for fine-tuning your base LLM.
*   **Advanced Retrieval-Augmented Generation (RAG):**
    *   Synthesizes answers to user questions by retrieving relevant facts from the knowledge base using a sophisticated, multi-stage pipeline for highly relevant and context-aware answers.
    *   **Temporal Reasoning:** Intelligently answers time-sensitive queries (e.g., "what is the newest iPhone?") by parsing date properties and prioritizing the most recent information.
*   **Pluggable Providers:** Supports different AI and storage providers (e.g., Gemini, local models, BigQuery, SQLite).
*   **Robust and Asynchronous:** Built with Tokio for efficient, non-blocking I/O.
*   **Identity & Ownership (`core-access` feature):** Provides a flexible user and ownership model. This allows the server to distinguish between content owned by different authenticated users and a shared "Guest User," ensuring clear data provenance.

### The Advanced RAG Pipeline

When you ask a question, the system uses a multi-stage process involving three LLM calls to deliver a precise answer:

1.  **Query Analysis (LLM Call #1):** The user's query is first analyzed to extract key entities (e.g., "iPhone") and keyphrases (e.g., "newest").

2.  **Multi-Stage Candidate Retrieval:**
    *   **Metadata Search:** A fast database query retrieves an initial set of candidate documents based on the extracted entities and keyphrases.
    *   **Keyword & Vector Search:** In parallel, keyword and vector searches are performed. The vector search uses an **Embedding Model (LLM Call #2)** to convert the user's query into a vector.

3.  **Reciprocal Rank Fusion (RRF):** The results from keyword and vector searches are intelligently combined and re-ranked using the RRF algorithm to produce a single, relevance-scored list.

4.  **Temporal Filtering:** The system checks the query for temporal keywords (like "newest," "latest"). If found, it filters the re-ranked list to find the single most recent document based on its date properties.

5.  **Answer Synthesis (LLM Call #3):** The final, highly-filtered context is passed to a powerful LLM, which generates a coherent, accurate answer based *only* on the provided information.

## Prerequisites

Before using this library, ensure you have the following:

1.  **Rust:** The Rust programming language and Cargo. You can install it from [rustup.rs](https://rustup.rs/).
2.  **Google Cloud Account:** An active Google Cloud account with a BigQuery project set up.
3.  **AI Provider API Key:** An API key for your chosen AI provider (e.g., Google Gemini or a local model).
4.  **GCP Authentication:** For local development, you must be authenticated with the Google Cloud SDK. Run the following command and follow the instructions:
    ```sh
    gcloud auth application-default login
    ```

## Configuration

The library is configured using environment variables. You can create a `.env` file in the root of the workspace or in this crate's directory (`crates/lib`).

**Required Environment Variables:**

*   `AI_API_KEY`: Your API key for a cloud-based AI provider (e.g., Google Gemini). Required if using the `gemini` provider.
*   `LOCAL_AI_API_URL`: The full API endpoint URL for a local AI provider (e.g., Ollama). Required if `AI_PROVIDER` is `local`.
*   `BIGQUERY_PROJECT_ID`: The ID of your Google Cloud project where BigQuery is enabled.
*   `EMBEDDINGS_API_URL`: The API endpoint for the text embeddings model (used for RAG).
*   `EMBEDDINGS_MODEL`: The name of the text embeddings model to use.

**Optional Environment Variables:**

*   `AI_PROVIDER`: The AI provider to use. Can be "gemini" or "local". Defaults to `gemini`.
*   `AI_MODEL`: The specific model name to use, which is mainly for the `local` provider.
*   `JINA_API_KEY`: An optional API key for the Jina Reader API to increase rate limits for web content fetching.
*   `RUST_LOG`: Sets the logging level for tracing. For example, `RUST_LOG=info,anyrag=debug`.

## Running Tests

You can run the tests for this specific crate from the workspace root:

```sh
cargo test -p anyrag
```

### Enabling Logs in Tests

To see detailed logs during test execution, you can set the `RUST_LOG` environment variable. This is incredibly helpful for debugging.

```sh
RUST_LOG=info cargo test -p anyrag -- --nocapture
```

The `-- --nocapture` flag tells the test runner to display the output immediately instead of capturing it. You can adjust the log level (e.g., `info`, `debug`, `trace`) as needed.