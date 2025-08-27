# AnyRag: Your Self-Improving Knowledge Base and RAG Engine

This project is a comprehensive Rust-based platform for building a self-improving knowledge base and interacting with your data—from data warehouses to live Google Sheets—using natural language.

## Core Features

-   **Natural Language to Data:**
    -   **Text-to-SQL:** Translates prompts into executable SQL queries for providers like Google BigQuery.
    -   **Dynamic Google Sheet Querying:** Automatically ingests a Google Sheet from a URL within a prompt and answers questions about its content on the fly.
    -   **Context-Aware:** Automatically injects the current date and time into the context, enabling time-sensitive questions like "What is the current hobby?".

-   **Comprehensive Knowledge Base Pipeline:**
    -   **Multi-Source Ingestion:** Builds a knowledge base from various sources:
        -   **Web URLs:** Fetches and cleans content from any webpage.
        -   **PDFs:** Ingests documents directly from file uploads or URLs.
        -   **Google Sheets:** Extracts structured Q&A pairs directly from sheets, respecting date ranges (`start_at`, `end_at`).
        -   **Raw Text:** Ingests and automatically chunks plain text.
    -   **AI-Powered Distillation:** Uses an LLM to automatically extract explicit Q&A pairs and generate new ones from unstructured text.
    -   **Vector Embeddings:** Generates embeddings for semantic search, enabling the RAG functionality.

-   **Retrieval-Augmented Generation (RAG):**
    -   Provides an API to ask questions against the knowledge base.
    -   Uses a hybrid search (vector + keyword) to find the most relevant facts.
    -   Synthesizes coherent, accurate answers using an LLM based on the retrieved context.

-   **Self-Improvement Cycle:**
    -   **Fine-Tuning Export:** Generates a dataset from the knowledge base in the correct format for fine-tuning your base LLM, which in turn improves future data extraction.

## API Response Structure

All JSON API responses follow a consistent structure for predictability. The primary content is always nested inside a `result` object.

### Debug Mode

All endpoints support a `debug` query parameter. When you append `?debug=true` to a request URL, the server adds a `debug` object to the response, containing contextual information about the request.

-   **Standard Response (`/ingest`)**
    ```json
    {
      "result": {
        "message": "Ingestion successful",
        "ingested_articles": 2
      }
    }
    ```
-   **Debug Response (`/ingest?debug=true`)**
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

The workspace is divided into two main crates. For detailed information on configuration, setup, and usage, please refer to the `README.md` file within each crate's directory.

-   **[`anyrag`](crates/lib/README.md)**: The core library containing all business logic. This includes prompt-to-SQL conversion, the knowledge base ingestion pipeline (fetch, distill, augment, store), and the RAG implementation.
-   **[`anyrag-server`](crates/server/README.md)**: A lightweight `axum` web server that exposes the library's functionality via a REST API. It provides endpoints for NL-to-SQL, knowledge base management, and RAG-based searches.

## Project Structure

```
anyrag/
├── Cargo.toml         # Workspace configuration
├── crates/
│   ├── lib/           # The core logic library
│   │   ├── Cargo.toml
│   │   ├── README.md  <-- Library details
│   │   └── src/
│   └── server/        # The axum web server
│       ├── Cargo.toml
│       ├── README.md  <-- Server details
│       ├── Dockerfile
│       └── src/
└── README.md          # This file
```

## Deployment to Google Cloud Run

This project includes a comprehensive script to automate deployment to Google Cloud Run. The script handles creating secrets, setting IAM permissions, building the container, and deploying the service.

### Prerequisites

-   The [Google Cloud SDK](https://cloud.google.com/sdk/docs/install) must be installed and initialized.
-   You must have a Google Cloud project with billing enabled.
-   Your `crates/server/.env` file must be created and contain your `AI_API_KEY` and `BIGQUERY_PROJECT_ID`.

### How to Deploy

1.  **Make the script executable:**
    ```sh
    chmod +x deploy.sh
    ```

2.  **Run the deployment script, passing your Google Cloud Project ID as an argument:**
    ```sh
    ./deploy.sh your-gcp-project-id
    ```
    The script will guide you through the authentication process and handle all the necessary steps to get your service live. Upon completion, it will output the URL for your deployed service.

## Running Tests

You can run all tests for the entire workspace from the root directory:

```sh
cargo test --workspace
```

For instructions on how to run tests for a specific crate, please see its respective `README.md`.

## License

This project is licensed under the MIT License.