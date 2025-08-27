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
-   `EMBEDDINGS_API_URL`: **(Required for RAG)** The URL for the text embeddings model.
-   `EMBEDDINGS_MODEL`: **(Required for RAG)** The name of the embeddings model.
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

## Running Examples

The server crate includes several examples in the `examples/` directory that demonstrate key features. You can run them from the workspace root.

**Example: Running the Knowledge Base RAG Workflow**
This example demonstrates the full end-to-end process of ingesting a URL, embedding the content, and asking questions against it.

```sh
# Ensure your .env file is configured, especially for the AI provider
RUST_LOG=info cargo run -p anyrag-server --example knowledge_prompt
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

Translates a natural language prompt into a query, executes it against a data warehouse (e.g., BigQuery), and formats the result. It is highly configurable and can also handle direct questions or ingest data from Google Sheets on the fly.

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

**Example:**
```sh
curl -X POST http://localhost:9090/knowledge/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://www.true.th/betterliv/support/true-app-mega-campaign"
  }'
```

#### `POST /ingest/file`

Ingests a PDF file directly. The server processes the PDF, uses an LLM to refine the extracted content into structured Markdown, stores this refined content, and then distills it into Q&A pairs for the knowledge base.

**Request Body:** `multipart/form-data`
- `file`: The PDF file to be ingested.
- `extractor`: (Optional) A string specifying the extraction strategy. Can be `"local"` (default) or `"gemini"`. The `local` strategy uses a pure Rust library to extract text and a generic LLM to refine it. The `gemini` strategy uses the multimodal Gemini API.

**Example:**
```sh
curl -X POST http://localhost:9090/ingest/file \
  -F "file=@/path/to/your/document.pdf" \
  -F "extractor=local"
```

#### `POST /ingest/pdf_url`

Downloads and ingests a PDF from a given URL. The server follows redirects, downloads the file, and then processes it using the same pipeline as the `/ingest/file` endpoint.

**Request Body:** `{"url": "...", "extractor": "..."}`
- `url`: The direct URL to the PDF file.
- `extractor`: (Optional) The extraction strategy. Can be `"local"` (default) or `"gemini"`.

**Example:**
```sh
curl -X POST http://localhost:9090/ingest/pdf_url \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://arxiv.org/pdf/2403.05530.pdf",
    "extractor": "local"
  }'
```

#### `POST /ingest/text`

Ingests raw text directly from the request body. The server will automatically chunk the text into smaller, manageable pieces based on paragraphs and a size limit, then store them in the `articles` table for later embedding and search.

**Request Body:** `{"text": "...", "source": "..."}`
- `text`: The raw text content to ingest.
- `source`: (Optional) A string to identify the origin of the text. Defaults to `text_input`.

**Example:**
```sh
curl -X POST http://localhost:9090/ingest/text \
  -H "Content-Type: application/json" \
  -d '{
    "text": "This is the first document about Rust macros.\n\nThis is a second paragraph about the same topic.",
    "source": "rust_docs_macros"
  }'
```

```

**Verifying the Ingestion**

After ingesting, you can use one of the search endpoints (like `/search/keyword`) to confirm that the text was stored.

```sh
curl -X POST http://localhost:9090/search/keyword \
  -H "Content-Type: application/json" \
  -d '{ "query": "macros" }'
```
This will return the chunks of text that contain the word "macros", proving the ingestion was successful.

#### `POST /embed/faqs/new`

Finds all FAQs in the knowledge base that have not yet been embedded and generates vector embeddings for them. This step is crucial for enabling semantic search.

**Request Body:** `{"limit": 100}` (Optional)

**Example:**
```sh
curl -X POST http://localhost:9090/embed/faqs/new \
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

**This is the primary RAG endpoint.** It takes a user's question, performs a semantic vector search to find the most relevant facts in the knowledge base, and then uses an LLM to synthesize a final, coherent answer based on that context.

**Request Body:** `{"query": "...", "limit": 5, "instruction": "..."}`
- `query`: The user's question.
- `limit`: (Optional) The number of facts to retrieve for context. Defaults to 5.
- `instruction`: (Optional) A specific instruction for the final LLM synthesis step.

**Example:**
```sh
curl -X POST http://localhost:9090/search/knowledge \
  -H "Content-Type: application/json" \
  -d '{
    "query": "ทำยังไงถึงจะได้เทสล่า",
    "instruction": "สรุปเงื่อนไขการรับสิทธิ์ลุ้นเทสล่า"
  }'
```

---

The following endpoints are for searching the legacy `articles` table, which is populated by the `/ingest` (RSS) endpoint.

#### `POST /search/hybrid`

Combines keyword and vector search for the most relevant results from the `articles` table.

**Example:**
```sh
curl -X POST http://localhost:9090/search/hybrid \
  -H "Content-Type: application/json" \
  -d '{ "query": "Qwen3" }'
```

#### `POST /search/vector` and `POST /search/keyword`

Perform pure semantic or keyword searches on the `articles` table, respectively.
