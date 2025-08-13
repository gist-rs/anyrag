# AnyRag: Your Self-Improving Knowledge Base and RAG Engine

This project is a comprehensive Rust-based platform for building a self-improving knowledge base and interacting with your data using natural language.

## Core Features

-   **Natural Language to SQL:** Translates user prompts into executable SQL queries for providers like Google BigQuery, allowing you to "talk" to your databases.
-   **Self-Improving Knowledge Base:** Implements a "virtuous cycle" for RAG:
    1.  **Ingest:** Fetches and cleans content from any URL.
    2.  **Distill & Augment:** Uses an LLM to automatically extract explicit Q&A pairs and generate new ones from unstructured text.
    3.  **Store:** Saves the structured knowledge into a local SQLite database, ready for retrieval.
    4.  **Export:** Generates a fine-tuning dataset from the knowledge base, allowing you to improve your base LLM, which in turn leads to better data extraction.
-   **Retrieval-Augmented Generation (RAG):** Provides an API endpoint to ask questions against the knowledge base. It retrieves the most relevant facts using vector search and uses an LLM to synthesize a coherent, accurate answer.

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