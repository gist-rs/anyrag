# `anyrag-server`

This crate provides a lightweight `axum` web server that exposes the `anyrag` library's functionality via a REST API. It allows you to translate natural language prompts into executable queries and execute them through HTTP requests.

## Features

*   **RESTful API:** Exposes all core functionalities via a simple API.
*   **Dynamic Sheet Querying:** Accepts Google Sheet URLs directly in prompts, ingesting and querying them on the fly.
*   **Multi-Source Ingestion:** Endpoints for building a knowledge base from web pages, PDFs (via upload or URL), raw text, and structured Google Sheets.
*   **RAG Endpoint:** A dedicated endpoint to ask questions against the knowledge base, using a hybrid search backend.
*   **Containerized Deployment:** Includes a multi-stage `Dockerfile` for building a minimal, secure server image.
*   **Asynchronous:** Built on top of Tokio for non-blocking, efficient request handling.
*   **Highly Configurable:** Uses a `config.yml` file for detailed control over AI providers and prompts.

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

The server now uses a powerful `config.yml` file for managing AI providers, embedding models, and task-specific prompts. This allows for much greater flexibility, such as using different models for different tasks (e.g., a fast local model for simple analysis and a powerful cloud model for complex synthesis).

### 1. Create your `config.yml`

Two templates are provided:
-   `config.gemini.yml`: Pre-configured for use with the Google Gemini API.
-   `config.local.yml`: Pre-configured for use with a local AI provider (like Ollama or LM Studio).

Copy the template that best matches your setup to a new file named `config.yml` in the `anyrag/crates/server` directory. This file is ignored by Git.

```sh
# For local models
cp config.local.yml config.yml

# For Google Gemini
cp config.gemini.yml config.yml
```

### 2. Configure your `.env` file

The `.env` file is now simplified and used only for secrets and environment-specific settings that you don't want to commit to version control. These variables are loaded and substituted into the `${VAR_NAME}` placeholders in your `config.yml`.

**Core Environment Variables:**

-   `AI_API_KEY`: **(Required for cloud providers)** Your secret API key.
-   `AI_API_URL`: The base URL for your primary AI provider.
-   `EMBEDDINGS_API_URL`: The URL for your text embedding model.
-   `PORT`: The port for the server to listen on. Defaults to `9090`.
-   `DB_URL`: The path to the SQLite database file. Defaults to `db/anyrag.db`.
-   `RUST_LOG`: The logging level (e.g., `info`, `debug`).
-   `JWT_SECRET`: A secret key for signing and validating JWTs. **It is highly recommended to set this in production.**

## Local Development (Without Docker)

For running the server directly on your machine for development.

1.  **Authenticate with GCP (Optional):** If you plan to use the BigQuery example, run `gcloud auth application-default login`.
2.  **Create `config.yml`:** Copy either `config.local.yml` or `config.gemini.yml` to `config.yml` and customize it for your needs.
3.  **Create `.env` File:** In the `anyrag/crates/server` directory, copy `.env.example` to `.env` and fill in your secrets (like `AI_API_KEY`).
4.  **Run the Server:**
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

### Step 2: Create Configuration Files

1.  **Create `config.yml`:** In the `anyrag/crates/server/` directory, copy your chosen template (`config.local.yml` or `config.gemini.yml`) to `config.yml`.
2.  **Create `.env` File:** In the same directory, copy `.env.example` to `.env` and add your secrets.

### Step 3: Place the Service Account Key (Optional)

If you plan to use the BigQuery example, place your downloaded Google Cloud service account key file in the **workspace root** (`anyrag/`) and name it `gcp_creds.json`.

### Step 4: Run the Docker Container

Execute this command from the **workspace root** (`anyrag/`). It mounts your configuration files into the container.

```sh
docker rm -f anyrag-server || true && \
docker run --rm -d \
  -p 9090:9090 \
  --env-file ./crates/server/.env \
  -v "$(pwd)/crates/server/config.yml:/app/config.yml:ro" \
  -v "$(pwd)/gcp_creds.json:/app/gcp_creds.json:ro" \
  --name anyrag-server \
  anyrag-server && \
docker logs -f anyrag-server
```

## Running Tests

To run the tests for this specific server crate, you can execute the following command from the workspace root:

```sh
cargo test -p anyrag-server
```

### Enabling Logs During Tests

For more detailed output during test runs, you can set the `RUST_LOG` environment variable. This is particularly useful for debugging.

```sh
RUST_LOG=info cargo test -p anyrag-server -- --nocapture
```

The `-- --nocapture` flag tells the test runner to display output from `println!` or `log` macros immediately, rather than capturing it and only showing it on test failure.

## API Endpoints

The server exposes a comprehensive set of endpoints for interacting with the `anyrag` library.

### Text-to-SQL API

This endpoint is for the core Natural Language to Query functionality.

#### `POST /prompt`

Translates a natural language prompt into a query, executes it, and formats the result. It can query a configured data warehouse (e.g., BigQuery) or, if a Google Sheet URL is detected in the prompt, it will dynamically ingest the sheet into a temporary SQLite table and query it instead.

**Request Body:** An `ExecutePromptOptions` JSON object.

**Example: Basic Query**
```sh
curl -X POST http://localhost:9090/prompt \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "tell me a joke"
  }'
```

### Knowledge Base Management API

These endpoints are for building and maintaining the self-improving knowledge base.

#### `POST /knowledge/ingest`

Triggers the full ingestion pipeline for a given URL. This process involves fetching the content, using an LLM to distill it into structured Q&A pairs, and storing it in the knowledge base.

**Request Body:** `{"url": "https://..."}`

**Note:** This is an authenticated endpoint. The `owner_id` of the ingested content will be automatically assigned based on the provided JWT. If no token is provided, it will be assigned to the "Guest User".

**Example:**
```sh
curl -X POST http://localhost:9090/knowledge/ingest \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "url": "https://www.true.th/betterliv/support/true-app-mega-campaign"
  }'
```

#### `POST /ingest/file`

Ingests a PDF file directly. The server processes the PDF, uses an LLM to refine the extracted content into structured Markdown, stores this refined content, and then distills it into Q&A pairs for the knowledge base.

**Request Body:** `multipart/form-data`
- `file`: The PDF file to be ingested.
- `extractor`: (Optional) A string specifying the extraction strategy. Can be `"local"` (default) or `"gemini"`.

**Note:** This is an authenticated endpoint. The `owner_id` is handled automatically.

**Example:**
```sh
curl -X POST http://localhost:9090/ingest/file \
  -H "Authorization: Bearer <your_jwt>" \
  -F "file=@/path/to/your/document.pdf" \
  -F "extractor=local"
```

#### `POST /ingest/pdf_url`

Downloads and ingests a PDF from a given URL. The server follows redirects, downloads the file, and then processes it using the same pipeline as the `/ingest/file` endpoint.

**Request Body:** `{"url": "...", "extractor": "..."}`
- `url`: The direct URL to the PDF file.
- `extractor`: (Optional) The extraction strategy. Can be `"local"` (default) or `"gemini"`.

**Note:** This is an authenticated endpoint. The `owner_id` is handled automatically.

**Example:**
```sh
curl -X POST http://localhost:9090/ingest/pdf_url \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "url": "https://arxiv.org/pdf/2403.05530.pdf",
    "extractor": "local"
  }'
```

#### `POST /ingest/sheet_faq`

Ingests Q&A pairs directly from a public Google Sheet. It's designed to handle structured FAQ data and can recognize date columns (`start_at`, `end_at`) to create time-sensitive knowledge.

**Request Body:** `{"url": "...", "gid": "...", "skip_header": true}`
- `url`: The public URL of the Google Sheet.
- `gid`: (Optional) The specific sheet/tab ID (the number after `gid=` in the URL).
- `skip_header`: (Optional) Whether to skip the first row of the sheet. Defaults to `true`.

**Note:** This is an authenticated endpoint. The `owner_id` is handled automatically.

**Example:**
```sh
curl -X POST http://localhost:9090/ingest/sheet_faq \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "url": "https://docs.google.com/spreadsheets/d/your_sheet_id/edit",
    "gid": "856666263"
  }'
```

#### `POST /ingest/text`

Ingests raw text directly from the request body. The server automatically chunks the text and stores each chunk as a document in the knowledge base.

**Request Body:** `{"text": "...", "source": "..."}`
- `text`: The raw text content to ingest.
- `source`: (Optional) A string to identify the origin of the text. Defaults to `text_input`.

**Note:** This is an authenticated endpoint. The `owner_id` is handled automatically.

**Example:**
```sh
curl -X POST http://localhost:9090/ingest/text \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "text": "This is the first document about Rust macros.\n\nThis is a second paragraph about the same topic.",
    "source": "rust_docs_macros"
  }'
```

#### `POST /embed/new`

Finds all documents in the knowledge base that have not yet been embedded and generates vector embeddings for them. This step is crucial for enabling semantic search.

**Request Body:** `{"limit": 100}` (Optional)

**Example:**
```sh
curl -X POST http://localhost:9090/embed/new \
  -H "Content-Type: application/json" \
  -d '{"limit": 50}'
```

#### `GET /knowledge/export`

Exports the entire FAQ knowledge base into a JSONL (JSON Lines) file suitable for fine-tuning a large language model. This completes the "virtuous cycle".

**Example:**
```sh
curl http://localhost:9090/knowledge/export -o finetuning_dataset.jsonl
```

### RAG & Search API

These endpoints are for searching the knowledge base.

#### `POST /search/knowledge`

**This is the primary RAG endpoint.** It takes a user's question, performs a hybrid search to find the most relevant facts in the knowledge base, and then uses an LLM to synthesize a final, coherent answer based on that context. The search is automatically filtered based on ownership: authenticated users see their own content plus guest content, while guest users see only guest content.

**Request Body:** `{"query": "...", "limit": 5, "instruction": "..."}`
- `query`: The user's question.
- `limit`: (Optional) The number of facts to retrieve for context. Defaults to 5.
- `instruction`: (Optional) A specific instruction for the final LLM synthesis step.

**Note:** This is an authenticated endpoint. The search results are automatically filtered based on the user's identity.

**Example:**
```sh
curl -X POST http://localhost:9090/search/knowledge \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "query": "ทำยังไงถึงจะได้เทสล่า",
    "instruction": "สรุปเงื่อนไขการรับสิทธิ์ลุ้นเทสล่า"
  }'
```
