# `anyrag-server`

This crate provides a lightweight `axum` web server that exposes the `anyrag` library's functionality via a REST API. It allows you to translate natural language prompts into executable queries and execute them through HTTP requests.

## Features

*   **RESTful API:** Exposes all core functionalities via a simple API.
*   **Dynamic Source Querying:** Accepts Google Sheet URLs, PDF URLs, or web page URLs directly in prompts, ingesting and querying them on the fly.
*   **Controllable Ingestion:** Endpoints for building knowledge bases from web pages, PDFs, raw text, Google Sheets, and **public GitHub repositories**.
*   **Advanced RAG Endpoints:** Dedicated endpoints for both knowledge bases (`/search/knowledge`) and code examples (`/search/examples`), using sophisticated, multi-stage hybrid search backends.
*   **Containerized Deployment:** Includes a multi-stage `Dockerfile` for building a minimal, secure server image.
*   **Asynchronous:** Built on top of Tokio for non-blocking, efficient request handling.
*   **Highly Configurable:** Uses a `config.yml` file for detailed control over AI providers, prompts, and features like temporal reasoning.

## Authentication

The server uses a flexible authentication model to support both multi-user deployments and simple, unauthenticated local use.

-   **JWT Authentication:** Authenticated endpoints expect a JSON Web Token (JWT) in the `Authorization` header.
    ```
    Authorization: Bearer <your_jwt_here>
    ```
-   **Guest User:** If no `Authorization` header is provided, the request is automatically processed as a special, deterministic **"Guest User"**. This allows for unauthenticated use (e.g., from a CLI or a local instance) while ensuring all ingested data has a clear owner.
-   **Security:** Providing an *invalid* or *expired* JWT will result in a `401 Unauthorized` error.

## Prerequisites

Before you begin, ensure you have the following:

1.  **Rust:** The Rust programming language and Cargo. You can install it from [rustup.rs](https://rustup.rs/).
2.  **Docker:** Required for building and running the containerized application.
3.  **Google Cloud Account:** An active Google Cloud account with a BigQuery project set up.
4.  **AI Provider API Key:** An API key for your chosen AI provider (e.g., Google Gemini).
5.  **Service Account Key:** A Google Cloud service account key file (JSON) is required for Docker authentication.

## IAM Permissions

The service account or user running the application needs the following IAM roles on your BigQuery project:

*   `roles/bigquery.dataViewer`: To inspect table schemas.
*   `roles/bigquery.jobUser`: To execute SQL queries.

You can grant these roles using the `gcloud` CLI.

## Configuration

The server uses a powerful, layered configuration system to maximize flexibility and adhere to the DRY (Don't Repeat Yourself) principle.

-   **`config.prompt.yml`**: Contains all the default task prompts. This file is part of the repository and provides a stable base.
-   **`config.yml`**: **(Required, user-created)** Defines AI providers, embedding models, and features like temporal reasoning. You create this file from a template (`config.gemini.yml` or `config.local.yml`).
-   **`prompt.yml`**: **(Optional, user-created)** Allows you to override specific prompts from the default set without modifying the base `config.prompt.yml` file. This file is ignored by Git.
-   **`.env`**: **(Required, user-created)** Stores secrets (like API keys) and environment-specific variables (like `PORT`).

### 1. Create your configuration files

1.  **Create `config.yml`:** Copy the template that best matches your setup to a new file named `config.yml` in the `anyrag/crates/server` directory.
    -   `config.gemini.yml`: For the Google Gemini API.
    -   `config.local.yml`: For a local AI provider (like Ollama or LM Studio).

    ```sh
    # For local models, this sets all tasks to use the 'local_default' provider
    cp crates/server/config.local.yml crates/server/config.yml
    ```
2.  **(Optional) Create `prompt.yml`:** If you want to customize any of the default prompts from `config.prompt.yml`, create a `prompt.yml` file and add *only* the `tasks` you wish to override.
3.  **(Optional) Configure Temporal Reasoning:** To enable the server to understand time-sensitive queries like "newest" or "latest", add the `temporal_reasoning` section to your `config.yml`.

    ```yaml
    # in config.yml
    temporal_reasoning:
      # Keywords that will trigger the temporal filtering logic.
      keywords: ["newest", "latest", "most recent"]
      # The name of the 'PROPERTY' in the content_metadata table that holds the date.
      property_name: "release_date"
    ```

### 2. Configure your `.env` file

The `.env` file is used for secrets and environment-specific settings. These variables are loaded and substituted into the `${VAR_NAME}` placeholders in your configuration files.

**Core Environment Variables:**

-   `AI_API_KEY`: **(Required for cloud providers)** Your secret API key.
-   `LOCAL_AI_API_URL`: The URL for your self-hosted or local AI provider.
-   `EMBEDDINGS_API_URL`: The URL for your text embedding model.
-   `JINA_API_KEY`: (Optional) An API key for Jina Reader to increase web scraping rate limits.
-   `PORT`: The port for the server to listen on. Defaults to `9090`.
-   `DB_URL`: The path to the SQLite database file. Defaults to `db/anyrag.db`.
-   `RUST_LOG`: The logging level (e.g., `info`, `debug`).
-   `JWT_SECRET`: A secret key for signing and validating JWTs. **It is highly recommended to set this in production.**

## Running Locally (Without Docker)

### 1. Running the Server

1.  **Create Configuration:** Ensure you have created your `crates/server/config.yml` and `crates/server/.env` files.
2.  **Run the Server:** From the **workspace root** (`anyrag/`), run the command:
    ```sh
    cargo run -p anyrag-server
    ```

### 2. Running the CLI (TUI)

The server must be running before you start the CLI.

1.  **Open a New Terminal.**
2.  **Run the CLI:** From the **workspace root** (`anyrag/`):
    ```sh
    cargo run -p cli
    ```

## Docker Deployment

### Step 1: Build the Docker Image

From the **workspace root** (`anyrag/`):
```sh
docker build -t anyrag-server -f crates/server/Dockerfile .
```

### Step 2: Create Configuration and Run

Ensure your `config.yml` and `.env` files are in the `anyrag/crates/server/` directory. Then, from the **workspace root**, run:
```sh
docker rm -f anyrag-server || true && \
docker run --rm -d \
  -p 9090:9090 \
  --env-file ./crates/server/.env \
  -v "$(pwd)/crates/server/config.yml:/app/config.yml:ro" \
  -v "$(pwd)/crates/server/prompt.yml:/app/prompt.yml:ro" \
  -v "$(pwd)/gcp_creds.json:/app/gcp_creds.json:ro" \
  --name anyrag-server \
  anyrag-server && \
docker logs -f anyrag-server
```

## Running Tests

To run the tests for this crate, execute from the workspace root:
```sh
cargo test -p anyrag-server
```
To enable logs during tests, use:
```sh
RUST_LOG=info cargo test -p anyrag-server -- --nocapture
```

## API Endpoints

The server exposes a comprehensive set of endpoints for interacting with the `anyrag` library. Key functionalities include:

*   **Knowledge Base Management:** Ingest from URLs, PDFs, text, and more.
*   **GitHub Ingestion:** Ingest code examples from public repositories.
*   **Advanced RAG Search:** Dedicated endpoints for querying both knowledge bases (`/search/knowledge`) and GitHub code examples (`/search/examples`).
*   **Advanced Data Generation:** Agentic workflows for generating content from retrieved context.

For detailed `curl` examples for every endpoint, please see the **[API Usage Examples documentation](../../EXAMPLES.md)**.