# `anyrag-server`

This crate provides a lightweight `axum` web server that exposes the `anyrag` library's functionality via a REST API. It allows you to translate natural language prompts into BigQuery SQL queries and execute them through HTTP requests.

## Features

*   **RESTful API:** Provides an easy-to-use API for integrations.
*   **Containerized Deployment:** Includes a multi-stage `Dockerfile` for building a minimal, secure server image.
*   **Asynchronous:** Built on top of Tokio for non-blocking, efficient request handling.
*   **Configurable:** Uses environment variables for easy configuration.

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

## Local Development (Without Docker)

For running the server directly on your machine for development.

1.  **Authenticate Locally:** Run `gcloud auth application-default login`.
2.  **Create `.env` File:** In the `anyrag/crates/server` directory, copy `.env.example` to `.env` and fill in your `AI_API_KEY`, `AI_API_URL`, and `BIGQUERY_PROJECT_ID`.
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

In the `anyrag/crates/server/` directory, copy the `.env.example` file to `.env` and add your secrets. This file is for variables that you don't want to type directly into the command line.

```sh
cp crates/server/.env.example crates/server/.env
```

Now, edit `crates/server/.env` and fill in these values:
*   `AI_API_KEY`
*   `BIGQUERY_PROJECT_ID`

### Step 3: Place the Service Account Key

Place your downloaded Google Cloud service account key file in the **workspace root** (`anyrag/`) and name it `admin.serviceAccount.json`.

### Step 4: Run the Docker Container

The following command is the most reliable way to run the container. It runs the server in the background, automatically removes any old container with the same name, and explicitly sets all critical configuration variables.

Execute this command from the **workspace root** (`anyrag/`):

```sh
docker rm -f anyrag-server || true && \
docker run --rm -d \
  -p 9090:8080 \
  --env-file ./crates/server/.env \
  -v "$(pwd)/admin.serviceAccount.json:/app/gcp_creds.json:ro" \
  --name anyrag-server \
  anyrag-server && \
docker logs -f anyrag-server
```

**Command Breakdown:**
*   `docker rm -f ...`: Force-removes any old `anyrag-server` container.
*   `--rm -d`: Runs the container in detached (background) mode and ensures it's removed when stopped.
*   `-p 9090:8080`: Maps your local port `9090` to the container's internal port `8080`.
*   `--env-file`: Loads the secrets from your `.env` file (`AI_API_KEY`, `BIGQUERY_PROJECT_ID`).
*   `-v ...`: Mounts the service account key into the container as **read-only**.
*   `-e GOOGLE_APPLICATION_CREDENTIALS=...`: **Crucially**, tells the app inside the container where to find the credentials.
*   `-e AI_API_URL=...`: Sets the AI provider endpoint, preventing connection errors.
*   `-e RUST_LOG="debug"`: Enables detailed logging so you can see the startup configuration.
*   `&& docker logs -f ...`: After starting, this immediately starts streaming the container's logs to your terminal. You can press `Ctrl+C` to stop viewing the logs at any time; the container will keep running.

## API Usage

Once the server is running, you can query it from another terminal. This example assumes the server is accessible on port `9090`.

```sh
curl -X POST http://localhost:9090/prompt \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "What is the total word_count for the corpus '\''kinghenryv'\''?",
    "table_name": "bigquery-public-data.samples.shakespeare",
    "instruction": "Use the data to provide a direct answer to the prompt. Form a natural-sounding sentence. Use thousand format for number."
  }'
```
