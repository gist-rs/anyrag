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

## Environment Variable Configuration

The server is configured using environment variables. For local development, you can create a `.env` file in this directory. For Docker, you can use an `--env-file` or pass variables with the `-e` flag.

### Core Configuration
-   `AI_API_KEY`: **(Required)** Your API key for the selected AI Provider.
-   `BIGQUERY_PROJECT_ID`: **(Required)** The ID of your Google Cloud project.
-   `AI_API_URL`: **(Required)** The full URL for the AI provider's API endpoint.
-   `AI_PROVIDER`: The AI provider to use. Can be "gemini" or "local". Defaults to "gemini".
-   `PORT`: The port for the server to listen on. Defaults to `8080`.
-   `RUST_LOG`: The logging level (e.g., `info`, `debug`).

### Prompt Customization (Optional)
You can set the following environment variables to define server-wide default prompts. This is useful for customizing the AI's behavior without changing the code. These can still be overridden by individual API requests.

-   `SYSTEM_PROMPT_TEMPLATE`: Sets the default system prompt for query generation. Use this to change the AI's core persona or rules.
-   `USER_PROMPT_TEMPLATE`: Sets the default user prompt template for query generation. You can use placeholders like `{language}`, `{context}`, `{prompt}`, and `{alias_instruction}`.

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

### Updating a Live Service on Google Cloud Run

If you have deployed this server as a service on Google Cloud Run, you can easily update its environment variables without a full redeployment. This is the recommended way to change the AI's behavior in a live environment by modifying the prompt templates.

Use the `gcloud run services update` command to apply changes.

**Example: Changing the AI's Persona**

This command updates the `SYSTEM_PROMPT_TEMPLATE` to make the AI act like a pirate.

```sh
gcloud run services update YOUR_SERVICE_NAME \
  --update-env-vars SYSTEM_PROMPT_TEMPLATE="You are a helpful pirate who always responds in pirate slang."
```

You can update multiple variables by providing a comma-separated list:

```sh
gcloud run services update YOUR_SERVICE_NAME \
  --update-env-vars KEY1=VALUE1,KEY2=VALUE2
```

This will trigger a new revision of your service with the updated environment.

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

### Advanced API Usage: Customizing Prompts

The `/prompt` endpoint is highly flexible, accepting any field from the `anyrag::ExecutePromptOptions` struct. This allows you to override the default AI behavior for both query generation and response formatting directly through the API. Any prompts provided in an API request will take precedence over server-wide defaults set via environment variables.

-   `system_prompt_template`: Bypasses the query generation logic entirely. Use this to make the AI perform generic tasks, like translation or summarization.
-   `format_system_prompt_template`: Overrides the default prompt for the final response formatting step, allowing you to control the style and tone of the output.

#### Example: Custom Formatting

This example uses `format_system_prompt_template` to make the AI act as a cheerful assistant that adds a winking face to its response.

```sh
curl -X POST http://localhost:9090/prompt \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "What is the total word_count for the corpus '\''kinghenryv'\''?",
    "table_name": "bigquery-public-data.samples.shakespeare",
    "instruction": "Answer with only the number.",
    "format_system_prompt_template": "You are a cheerful AI assistant. You always add a winking face ;) at the end of your response."
  }'
```
