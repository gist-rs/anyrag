# `anyrag-github` Crate

This crate provides all functionality related to ingesting and searching code examples from public GitHub repositories for the `anyrag` ecosystem. It is used by both `anyrag-cli` for manual ingestion and `anyrag-server` for the code search RAG API.

## Features

*   **GitHub Ingestion Pipeline:**
    *   **Repository Crawler:** Clones public repositories, handling versioning via tags, branches, or commits. If no version is specified, it intelligently discovers the latest semantic version tag.
    *   **Intelligent Extractor:** Finds code examples from tests, doc comments, dedicated example files, and READMEs, prioritizing sources to ensure accuracy and relevance.
    *   **Idempotent Storage:** Stores versioned examples in a dedicated database for each repository, ensuring that re-ingesting a version correctly updates its content without duplication.
*   **Advanced Code Example Search (RAG):**
    *   Retrieves relevant code snippets from multiple repositories using an advanced hybrid search that combines metadata pre-filtering, keyword search, and vector search for maximum relevance and performance.

### The Advanced RAG Pipeline for Code Search

The system uses a multi-stage process to deliver a precise answer:

1.  **Query Analysis (LLM Call #1):** The user's query is first analyzed to extract key entities (e.g., "turso client", "connect function") and keyphrases (e.g., "connect to database", "initialize client").

2.  **Multi-Stage Candidate Retrieval:**
    *   **Metadata Pre-filtering:** A fast initial search finds examples that contain the extracted entities, creating a small, relevant candidate pool. This dramatically improves the performance and accuracy of the subsequent steps.
    *   **Keyword & Vector Search:** In parallel, keyword and vector searches are performed across the filtered code examples. The vector search uses an **Embedding Model (LLM Call #2)** to convert the user's query into a vector for semantic matching.

3.  **Reciprocal Rank Fusion (RRF):** The results from keyword and vector searches are intelligently combined and re-ranked using the RRF algorithm to produce a single, relevance-scored list.

4.  **Answer Synthesis (LLM Call #3):** The final, highly-filtered context is passed to a powerful LLM, which generates a coherent, accurate answer based *only* on the provided code examples.

## Prerequisites

Before using this crate's functionality, please ensure you have the following set up:

1.  **Rust**: The Rust toolchain is required. You can install it from [rustup.rs](https://rustup.rs/).
2.  **Git**: The `git` command-line tool must be installed and available in your system's PATH. This is required for cloning repositories.

## Usage (via `anyrag-cli`)

The primary way to use this crate's ingestion capabilities is through the `dump github` command in `anyrag-cli`.

### `dump github`

Clones a public GitHub repository, intelligently extracts code examples from documentation, tests, and example files, stores them in a local database, and generates a consolidated Markdown file for use as LLM context.

**Arguments:**

*   `<URL>`: **(Required)** The full URL of the public GitHub repository (e.g., `https://github.com/tursodatabase/turso`).
*   `--version <VERSION>`: (Optional) A specific git tag, branch, or commit hash to check out. If omitted, the CLI will automatically use the latest semantic version tag it finds.
*   `--embedding-api-url <URL>`: (Optional) The API endpoint for a text embedding model. If provided, embeddings will be generated for all extracted examples, enabling vector search capabilities. Can also be set via the `EMBEDDINGS_API_URL` environment variable.
*   `--embedding-model <MODEL_NAME>`: (Required if `--embedding-api-url` is set) The name of the embedding model to use (e.g., `text-embedding-ada-002`). Can also be set via the `EMBEDDINGS_MODEL` environment variable.
*   `--no-process`: (Optional) Disables the final automatic step of chunking the generated Markdown context file.

**Examples:**

**1. Basic Ingestion:**

This command will ingest the Turso repository, find all relevant examples, generate a context file, and automatically process that file into a chunked database.
```sh
cargo run -p cli -- dump github https://github.com/tursodatabase/turso
```

**2. Ingestion with Embeddings:**

This command does everything the basic command does, but it also generates vector embeddings for each extracted code example and for each chunk of the final context file. This is required for enabling semantic vector search.
```sh
cargo run -p cli -- dump github https://github.com/tursodatabase/turso \
  --embedding-api-url "http://localhost:1234/api/embeddings" \
  --embedding-model "text-embedding-qwen3-embedding-8b"
```

**Expected Output:**

After a successful run, you will see messages indicating the number of examples ingested. A new context file will be created (e.g., `tursodatabase-turso-v0.90.1-context.md`), and you will see a final confirmation that the chunks from this file have also been stored in a local database (e.g., `db/github_chunks/tursodatabase-turso.db`).

## Running Tests

You can run the tests for this specific crate from the workspace root:

```sh
cargo test -p github
```
