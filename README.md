# AnyRag: Your Self-Improving Knowledge Base and RAG Engine

This project is a comprehensive Rust-based platform for building a self-improving knowledge base and interacting with your data—from data warehouses to live Google Sheets—using natural language.

## Core Features

-   **Natural Language to Data:**
    -   **Text-to-SQL:** Translates prompts into executable SQL queries for providers like Google BigQuery.
    -   **Dynamic Google Sheet Querying:** Automatically ingests a Google Sheet from a URL within a prompt and answers questions about its content on the fly.
    -   **Context-Aware:** Automatically injects the current date and time into the context, enabling time-sensitive questions.

-   **Comprehensive Knowledge Base Pipeline:**
    -   **Multi-Source Ingestion:** Builds a knowledge base from various sources:
        -   Web URLs, PDFs, Google Sheets, RSS Feeds, and Raw Text.
    -   **AI-Powered Distillation:** Uses an LLM to automatically extract explicit Q&A pairs and generate new ones from unstructured text.
    -   **Vector Embeddings:** Generates embeddings for semantic search.

-   **Advanced Retrieval-Augmented Generation (RAG):**
    -   Provides an API to ask questions against the knowledge base.
    -   Employs a sophisticated, multi-stage pipeline for highly relevant and context-aware answers.
    -   **Temporal Reasoning:** Understands and correctly answers time-sensitive queries like "what is the newest..." by filtering results based on date properties.

-   **Self-Improvement Cycle:**
    -   **Fine-Tuning Export:** Generates a dataset from the knowledge base in the correct format for fine-tuning your base LLM, which in turn improves future data extraction.

-   **Flexible Identity & Ownership:**
    -   **JWT & Guest Access:** Supports standard JWT-based authentication and seamlessly falls back to a deterministic "Guest User," ensuring all ingested data has a clear owner without requiring a login.
    -   **Ownership-Aware Search:** Search results are automatically and securely filtered based on the user's identity.

## The Advanced RAG Pipeline

When you ask a question, the system uses a multi-stage process involving three LLM calls to deliver a precise answer:

1.  **Query Analysis (LLM Call #1):** The user's query is first analyzed to extract key entities (e.g., "iPhone") and keyphrases (e.g., "newest").

2.  **Multi-Stage Candidate Retrieval:**
    *   **Metadata Search:** A fast database query retrieves an initial set of candidate documents based on the extracted entities and keyphrases.
    *   **Keyword & Vector Search:** In parallel, keyword and vector searches are performed. The vector search uses an **Embedding Model (LLM Call #2)** to convert the user's query into a vector.

3.  **Reciprocal Rank Fusion (RRF):** The results from keyword and vector searches are intelligently combined and re-ranked using the RRF algorithm to produce a single, relevance-scored list.

4.  **Temporal Filtering:** The system checks the query for temporal keywords (like "newest," "latest"). If found, it filters the re-ranked list to find the single most recent document based on its date properties.

5.  **Answer Synthesis (LLM Call #3):** The final, highly-filtered context is passed to a powerful LLM, which generates a coherent, accurate answer based *only* on the provided information.

## API Response Structure

All JSON API responses follow a consistent `result` object structure. Appending `?debug=true` to any request URL will add a `debug` object to the response with contextual information.

-   **Standard Response (`/ingest/rss`)**
    ```json
    {
      "result": {
        "message": "Ingestion successful",
        "ingested_articles": 2
      }
    }
    ```
-   **Debug Response (`/ingest/rss?debug=true`)**
    ```json
    {
      "debug": {
        "url": "http://example.com/rss"
      },
      "result": {
        "message": "Ingestion successful",
        "ingested_articles": 2
      }
    }
    ```

## Workspace Crates

The workspace is divided into two main crates. For detailed information, please refer to the `README.md` file within each crate's directory.

-   **[`anyrag`](crates/lib/README.md)**: The core library containing all business logic.
-   **[`anyrag-server`](crates/server/README.md)**: A lightweight `axum` web server that exposes the library's functionality via a REST API.

## API Examples

For detailed `curl` examples for every API endpoint, please see the **[API Usage Examples (EXAMPLES.md)](EXAMPLES.md)** document.

## Project Structure

```
anyrag/
├── Cargo.toml         # Workspace configuration
├── EXAMPLES.md        # Detailed API usage examples
├── crates/
│   ├── lib/           # The core logic library
│   │   ├── README.md  <-- Library details
│   │   └── src/
│   └── server/        # The axum web server
│       ├── README.md  <-- Server details
│       ├── Dockerfile
│       └── src/
└── README.md          # This file
```

## Deployment to Google Cloud Run

This project includes a comprehensive script (`deploy.sh`) to automate deployment to Google Cloud Run.

### Prerequisites

-   The [Google Cloud SDK](https://cloud.google.com/sdk/docs/install) is installed and initialized.
-   You have a Google Cloud project with billing enabled.
-   Your `crates/server/.env` file contains your `AI_API_KEY` and `BIGQUERY_PROJECT_ID`.

### How to Deploy

1.  **Make the script executable:** `chmod +x deploy.sh`
2.  **Run the deployment script:** `./deploy.sh your-gcp-project-id`

The script will guide you through the process and output the URL for your deployed service.

## Running Tests

You can run all tests for the entire workspace from the root directory:

```sh
cargo test --workspace
```

## License

This project is licensed under the MIT License.