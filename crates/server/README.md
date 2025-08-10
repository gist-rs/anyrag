# `anyrag-server`

This crate provides a lightweight `axum` web server that exposes the `anyrag` library's functionality via a REST API. It allows you to translate natural language prompts into executable queries and execute them through HTTP requests.

## Features

*   **RESTful API:** Provides an easy-to-use API for integrations.
*   **Containerized Deployment:** Includes a multi-stage `Dockerfile` for building a minimal, secure server image.
*   **Asynchronous:** Built on top of Tokio for non-blocking, efficient request handling.
*   **Highly Configurable:** Uses environment variables for easy configuration of prompts and providers.

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

## Environment Variable Configuration

The server is configured using environment variables. For local development, you can create a `.env` file in this directory. For Docker, you can use an `--env-file` or pass variables with the `-e` flag.

### Core Configuration
-   `AI_API_KEY`: **(Required)** Your API key for the selected AI Provider.
-   `BIGQUERY_PROJECT_ID`: **(Required)** The ID of your Google Cloud project.
-   `AI_API_URL`: **(Required)** The full URL for the AI provider's API endpoint.
-   `AI_PROVIDER`: The AI provider to use. Can be "gemini" or "local". Defaults to "gemini".
-   `PORT`: The port for the server to listen on. Defaults to `9090`.
-   `RUST_LOG`: The logging level (e.g., `info`, `debug`).

### Prompt Customization (Optional)
You can set the following environment variables to define server-wide default prompts. This is useful for customizing the AI's behavior without changing the code. These can still be overridden by individual API requests.

-   `QUERY_SYSTEM_PROMPT_TEMPLATE`: Controls the AI's core instructions for **query generation**. Use this to change its persona, add strict rules, or adapt to a new query language.
-   `QUERY_USER_PROMPT_TEMPLATE`: Controls how the user's question and table context are presented to the AI for **query generation**. Available placeholders: `{language}`, `{context}`, `{prompt}`, `{alias_instruction}`.
-   **`FORMAT_SYSTEM_PROMPT_TEMPLATE`**: Controls the AI's persona for the final response formatting step.
-   **`FORMAT_USER_PROMPT_TEMPLATE`**: Controls how the data and original prompt are presented to the AI for formatting. Available placeholders: `{prompt}`, `{instruction}`, `{content}`.

## Local Development (Without Docker)

For running the server directly on your machine for development.

1.  **Authenticate Locally:** Run `gcloud auth application-default login`.
2.  **Create `.env` File:** In the `anyrag/crates/server` directory, copy `.env.example` to `.env` and fill in your secrets and any desired prompt customizations.
3.  **Run the Server:**
    ```sh
    cargo run
    ```

## Docker Deployment

This is the recommended way to run the server in a production-like environment.

### Step 1: Build the Docker Image

From the **workspace root** (`anyrag/`), run the build command:

```sh
docker build -t anyrag-server -f crates/server/Dockerfile .
```

### Step 2: Create the `.env` File

In the `anyrag/crates/server/` directory, copy the `.env.example` file to `.env` and add your secrets.

### Step 3: Place the Service Account Key

Place your downloaded Google Cloud service account key file in the **workspace root** (`anyrag/`) and name it `gcp_creds.json`.

### Step 4: Run the Docker Container

Execute this command from the **workspace root** (`anyrag/`):

```sh
docker rm -f anyrag-server || true && \
docker run --rm -d \
  -p 9090:9090 \
  --env-file ./crates/server/.env \
  -v "$(pwd)/gcp_creds.json:/app/gcp_creds.json:ro" \
  --name anyrag-server \
  anyrag-server && \
docker logs -f anyrag-server
```

### Updating a Live Service on Google Cloud Run

If you have deployed this server as a service on Google Cloud Run, you can easily update its environment variables without a full redeployment. This is the recommended way to change the AI's behavior in a live environment by modifying the prompt templates.

Use the `gcloud run services update` command to apply changes.

**Example: Changing the AI's Query Generation Persona**

```sh
gcloud run services update YOUR_SERVICE_NAME \
  --update-env-vars QUERY_SYSTEM_PROMPT_TEMPLATE="You are a SQL expert who only writes queries using Common Table Expressions (CTEs)."
```

## API Endpoints

The server exposes several endpoints for interacting with the `anyrag` library. All `POST` endpoints expect a JSON body and return a JSON response.

### Prompt API

This endpoint is for the core Natural Language to Query functionality.

#### `POST /prompt`

Translates a natural language prompt into a query, executes it against the storage provider (e.g., BigQuery), and formats the result. It is highly configurable.

**Request Body:** An `ExecutePromptOptions` JSON object.

**Example: Basic Query**
```sh
curl -X POST http://localhost:9090/prompt \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "What is the total word_count for the corpus '\''kinghenryv'\''?",
    "table_name": "bigquery-public-data.samples.shakespeare"
  }'
```

### Search API

These endpoints are for searching articles ingested into the local SQLite database. They provide different search strategies to suit various needs.

**Common Request Body:**
```json
{
  "query": "your search query",
  "limit": 10
}
```
- `query`: The text you want to search for.
- `limit`: (Optional) The maximum number of results to return. Defaults to 10.

---

#### `POST /search/keyword`

Performs a fast, traditional keyword search using a Full-Text Search (FTS) index. This is best for finding exact words or phrases.

**Example:**
```sh
curl -X POST http://localhost:9090/search/keyword \
  -H "Content-Type: application/json" \
  -d '{
    "query": "PostgreSQL performance",
    "limit": 5
  }'
```

---

#### `POST /search/vector`

Performs a semantic or conceptual search using vector embeddings. This is best for finding articles that are thematically similar to the query, even if they don't contain the exact keywords.

**Example:**
```sh
curl -X POST http://localhost:9090/search/vector \
  -H "Content-Type: application/json" \
  -d '{
    "query": "building web applications with Rust",
    "limit": 5
  }'
```

---

#### `POST /search/hybrid`

Combines the strengths of both keyword and vector search using a Reciprocal Rank Fusion (RRF) algorithm. It provides the most balanced and often the most relevant results. **This is the recommended endpoint for general-purpose search.**

**Example:**
```sh
curl -X POST http://localhost:9090/search/hybrid \
  -H "Content-Type: application/json" \
  -d '{
    "query": "Qwen3",
    "limit": 5
  }'
```
