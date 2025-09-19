# `github` Crate

This crate provides all functionality related to ingesting and searching code examples from public GitHub repositories for the `anyrag` ecosystem. It is used by `anyrag-cli` for manual ingestion and `anyrag-server` for the code example search API.

## Features

*   **GitHub Ingestion Pipeline:**
    *   **Repository Crawler:** Clones public repositories, handling versioning via tags or branches. If no version is specified, it intelligently infers the version from `Cargo.toml`.
    *   **Intelligent Extractor:** Finds Rust code examples from doc comments (`///`, `//!`), `#[doc]` attributes, `README.md`, and files under `examples/` and `tests/`.
    *   **Versioned Storage:** Stores extracted examples in a dedicated, version-specific SQLite database for each repository, ensuring that re-ingesting a version correctly updates its content without duplication.
    *   **Automatic Embedding:** Automatically generates vector embeddings for each code snippet during ingestion, enabling semantic search.

*   **Advanced Code Example Search (RAG):**
    *   Provides an API endpoint (`/search/examples`) to retrieve relevant code snippets from one or more ingested repositories.
    *   Uses a sophisticated hybrid search that combines keyword and vector search for maximum relevance.

## The RAG Pipeline for Code Search

The system uses a multi-stage process to find the most relevant code examples for a user's query. Unlike the knowledge base pipeline, this process does **not** synthesize a final answer; it returns a ranked list of the best code snippets.

1.  **Query Analysis (LLM Call):** The user's query is analyzed by an LLM specialized in understanding code-related questions. It extracts key **entities** (like function or library names) and **keyphrases** (like "how to connect").

2.  **Parallel Candidate Retrieval:** The search is performed concurrently across all specified repository databases:
    *   **Vector Search:** The user's query is converted into a vector **(Embedding Model Call)** to find semantically similar code snippets.
    *   **Keyword Search:** A traditional keyword search is run against the code snippets to find exact matches.

3.  **Reciprocal Rank Fusion (RRF):** The results from both the vector and keyword searches are intelligently combined and re-ranked using the RRF algorithm. This produces a single, relevance-scored list of the best matching code examples.

4.  **Direct Results:** The final, ranked list of `SearchResult` objects is returned directly to the client.

## Usage (via `anyrag-cli`)

The primary way to use this crate's ingestion capabilities is through the `dump github` command in `anyrag-cli`.

### `dump github`

Clones a public GitHub repository, extracts all Rust code examples, and stores them in a versioned, local SQLite database under `db/github_ingest/<repo_name>.db`.

**Arguments:**

*   `--url <URL>`: **(Required)** The URL of the public GitHub repository to clone.
*   `--version <VERSION>`: (Optional) A specific git tag or commit hash to check out. If omitted, the version will be inferred from the `version` field in `Cargo.toml`.
*   `--embedding-api-url <URL>`: (Optional) The API endpoint for a text embedding model.
*   `--embedding-model <MODEL_NAME>`: (Required if `--embedding-api-url` is set) The name of the embedding model to use.

**Examples:**
```sh
cargo run -p cli dump github \
  --url https://github.com/rust-lang/book \
  --version v2.0.0 \
  --embedding-api-url "http://localhost:1234/v1/embeddings" \
  --embedding-model "text-embedding-qwen3-embedding-8b"
```

```sh
cargo run -p cli dump github \
  --url https://github.com/tursodatabase/turso \
  --version v0.1.5 \
  --embedding-api-url "http://localhost:1234/v1/embeddings" \
  --embedding-model "text-embedding-qwen3-embedding-8b"
```

## Running Tests

You can run the tests for this specific crate from the workspace root:

```sh
cargo test -p github
```
