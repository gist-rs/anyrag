# `anyrag-server`

This crate provides a lightweight `axum` web server that exposes the `anyrag` library's functionality via a REST API. It allows you to translate natural language prompts into executable queries and execute them through HTTP requests.

## Features

*   **RESTful API:** Exposes all core functionalities via a simple API.
*   **Dynamic Source Querying:** Accepts Google Sheet URLs, PDF URLs, or web page URLs directly in prompts, ingesting and querying them on the fly.
*   **Controllable Ingestion:** Endpoints for building a knowledge base from web pages, PDFs, raw text, and Google Sheets, with fine-grained control over AI-based FAQ generation and embedding.
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

The server uses a powerful, layered configuration system to maximize flexibility and adhere to the DRY (Don't Repeat Yourself) principle.

-   **`config.prompt.yml`**: Contains all the default task prompts. This file is part of the repository and provides a stable base.
-   **`config.yml`**: **(Required, user-created)** Defines which AI providers and embedding models to use. You create this file from a template (`config.gemini.yml` or `config.local.yml`).
-   **`prompt.yml`**: **(Optional, user-created)** Allows you to override specific prompts from the default set without modifying the base `config.prompt.yml` file. This file is ignored by Git.
-   **`.env`**: **(Required, user-created)** Stores secrets (like API keys) and environment-specific variables (like `PORT`).

This layered approach allows you to customize providers and prompts independently while keeping the default configuration clean.

### 1. Create your configuration files

1.  **Create `config.yml`:** Copy the template that best matches your setup to a new file named `config.yml` in the `anyrag/crates/server` directory. This file is for defining your AI providers and is ignored by Git.
    -   `config.gemini.yml`: For the Google Gemini API.
    -   `config.local.yml`: For a local AI provider (like Ollama or LM Studio).

    ```sh
    # For local models, this sets all tasks to use the 'local_default' provider
    cp crates/server/config.local.yml crates/server/config.yml

    # For Google Gemini
    # cp crates/server/config.gemini.yml crates/server/config.yml
    ```
2.  **(Optional) Create `prompt.yml`:** If you want to customize any of the default prompts from `config.prompt.yml`, create a `prompt.yml` file and add *only* the `tasks` you wish to override. For example:
    ```yml
    # prompt.yml
    tasks:
      rag_synthesis:
        provider: "local_fast" # You can even change the provider for a specific task
        system_prompt: "You are a pirate AI. Answer the user's question based on the scrolls."
    ```

### 2. Configure your `.env` file

The `.env` file is used for secrets and environment-specific settings that you don't want to commit to version control. These variables are loaded and substituted into the `${VAR_NAME}` placeholders in your configuration files (`config.yml`, `prompt.yml`, etc.).

**Core Environment Variables:**

-   `AI_API_KEY`: **(Required for cloud providers)** Your secret API key.
-   `AI_API_URL`: The base URL for your primary AI provider.
-   `EMBEDDINGS_API_URL`: The URL for your text embedding model.
-   `PORT`: The port for the server to listen on. Defaults to `9090`.
-   `DB_URL`: The path to the SQLite database file. Defaults to `db/anyrag.db`.
-   `RUST_LOG`: The logging level (e.g., `info`, `debug`).
-   `JWT_SECRET`: A secret key for signing and validating JWTs. **It is highly recommended to set this in production.**

## Running Locally (Without Docker)

For running the server and CLI directly on your machine for development.

### 1. Running the Server

The server is the backend API that the CLI connects to.

1.  **Create Configuration:** Ensure you have created your `crates/server/config.yml` and `crates/server/.env` files as described in the "Configuration" section above.
2.  **Run the Server:** From the **workspace root** (`anyrag/`), run the command:
    ```sh
    cargo run -p anyrag-server
    ```
The server will start and listen on the port specified in your `.env` file (defaults to 9090).

### 2. Running the CLI (TUI)

The CLI is the interactive terminal user interface. The server must be running before you start the CLI.

1.  **Open a New Terminal:** Keep the server running in its own terminal window.
2.  **Run the CLI:** From the **workspace root** (`anyrag/`), run the command in the new terminal:
    ```sh
    cargo run -p cli
    ```
The interactive TUI will launch and connect to the running server.

## Docker Deployment

This is the recommended way to run the server in a production-like environment.

### Step 1: Build the Docker Image

From the **workspace root** (`anyrag/`), run the build command:

```sh
docker build -t anyrag-server -f crates/server/Dockerfile .
```

### Step 2: Create Configuration Files

1.  **Create `config.yml`:** In the `anyrag/crates/server/` directory, copy your chosen template to `config.yml`.
2.  **Create `prompt.yml` (Optional):** If you have prompt overrides, ensure this file is present.
3.  **Create `.env` File:** In the same directory, copy `.env.example` to `.env` and add your secrets.

### Step 3: Place the Service Account Key (Optional)

If you plan to use the BigQuery example, place your downloaded Google Cloud service account key file in the **workspace root** (`anyrag/`) and name it `gcp_creds.json`.

### Step 4: Run the Docker Container

Execute this command from the **workspace root** (`anyrag/`). It mounts your local configuration files into the container. Note that `config.prompt.yml` is already included in the Docker image.

```sh
# This command mounts both your main config and your optional prompt overrides.
# If you don't have a prompt.yml, you can remove that '-v' line.
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

**Example: Shorthand Alias**
```sh
# This shorthand...
curl -X POST http://localhost:9090/prompt \
  -H "Content-Type: application/json" \
  -d '{
    "db": "kratooded",
    "prompt": "ls pantip_topics_samples limit=20"
  }'
# ...is automatically translated into a full prompt for the AI.
```

### Knowledge Base Management API

These endpoints are for building and maintaining the self-improving knowledge base.

#### `POST /ingest/web`

Fetches and processes content from a web URL.

**Query Parameters:**
- `faq` (boolean, optional): If `true`, runs the full AI-based pipeline to distill the content into structured Q&A pairs. Defaults to `false`.
- `embed` (boolean, optional): If `true` (default), generates and stores vector embeddings for the ingested content, making it available for semantic search.

**Request Body:** `{"url": "https://..."}`

**Example (Light Ingest):**
```sh
curl -X POST http://localhost:9090/ingest/web \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "url": "https://www.true.th/betterliv/support/true-app-mega-campaign"
  }'
```

**Example (FAQ Generation):**
```sh
curl -X POST "http://localhost:9090/ingest/web?faq=true" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "url": "https://www.true.th/betterliv/support/true-app-mega-campaign"
  }'
```

#### `POST /ingest/rss`

Ingests articles from an RSS feed URL, storing each item as a separate document.

**Request Body:** `{"url": "https://..."}`

**Example:**
```sh
curl -X POST http://localhost:9090/ingest/rss \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "url": "http://example.com/feed.xml"
  }'
```

#### `POST /ingest/pdf`

Processes a PDF from either a direct file upload or a URL.

**Query Parameters:**
- `faq` (boolean, optional): If `true`, runs the full AI-based pipeline to distill the content into structured Q&A pairs. Defaults to `false`.
- `embed` (boolean, optional): If `true` (default), generates vector embeddings for the ingested content.

**Request Body:** `multipart/form-data` containing either a `file` part or a `url` part.
- `file`: The PDF file to be ingested.
- `url`: A direct URL to a PDF file to be downloaded and ingested.
- `extractor`: (Optional) A string specifying the extraction strategy. Can be `"local"` (default) or `"gemini"`.

**Example (File Upload):**
```sh
curl -X POST "http://localhost:9090/ingest/pdf?faq=true" \
  -H "Authorization: Bearer <your_jwt>" \
  -F "file=@/path/to/your/document.pdf" \
  -F "extractor=local"
```

**Example (URL):**
```sh
curl -X POST "http://localhost:9090/ingest/pdf?faq=true" \
  -H "Authorization: Bearer <your_jwt>" \
  -F "url=https://arxiv.org/pdf/2403.05530.pdf" \
  -F "extractor=local"
```

#### `POST /ingest/sheet`

Ingests data from a public Google Sheet. The behavior is controlled by the `faq` query parameter.

**Query Parameters:**
- `faq` (boolean, optional): If `true`, ingests a sheet formatted with "Question" and "Answer" columns directly as Q&A pairs. If `false` (default), ingests the sheet as a generic table in the database.
- `embed` (boolean, optional): If `true` (default), generates vector embeddings for the ingested rows or Q&A pairs.

**Request Body:** `{"url": "...", "gid": "...", "skip_header": true}`

**Example (Generic Table Ingest):**
```sh
curl -X POST http://localhost:9090/ingest/sheet \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "url": "https://docs.google.com/spreadsheets/d/your_sheet_id/edit"
  }'
```

**Example (FAQ Ingest):**
```sh
curl -X POST "http://localhost:9090/ingest/sheet?faq=true" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "url": "https://docs.google.com/spreadsheets/d/your_sheet_id/edit",
    "gid": "856666263"
  }'
```

#### `POST /ingest/text`

Ingests raw text directly from the request body.

**Query Parameters:**
- `faq` (boolean, optional): If `true`, runs the full AI-based pipeline on the entire text. If `false` (default), the text is automatically chunked and each chunk is stored as a separate document.
- `embed` (boolean, optional): If `true` (default), generates vector embeddings for the ingested text.

**Request Body:** `{"text": "...", "source": "..."}`

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

Exports the entire FAQ knowledge base into a JSONL (JSON Lines) file suitable for fine-tuning a large language model.

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

### Advanced API

These endpoints provide more direct control over data retrieval and generation, catering to advanced use cases.

#### `POST /db/query`

Executes a raw, read-only SQL query directly against a project's database. This is useful for programmatic access where you know the exact query you need to run.

**Request Body:** `{"db": "...", "query": "..."}`

**Example:**
```sh
curl -X POST http://localhost:9090/db/query \
  -H "Content-Type: application/json" \
  -d '{
    "db": "kratooded",
    "query": "SELECT _id, title, rating FROM pantip_topics_samples WHERE rating >= 3 ORDER BY rating DESC LIMIT 10"
  }'
```

#### `POST /gen/text`

Generates new text content by first executing the `context_prompt` as a Text-to-SQL query to retrieve structured data, and then feeding that data as context to the `generation_prompt`. This allows for creating sophisticated, data-driven content.

**Request Body:** `{"db": "...", "generation_prompt": "...", "context_prompt": "..."}`

**Example 1:**
```sh
curl -X POST http://localhost:9090/gen/text \
  -H "Content-Type: application/json" \
  -d '{
    "db": "kratooded",
    "generation_prompt": "User''s GoalWrite a short, romantic story in the style of a modern Thai drama. The story must be in Thai language (ภาษาไทย) only, told from a first-person perspective using \"ผม\" (male) or \"เรา\" (female) to make it feel personal and intimate. Aim for 800-1500 characters to keep it concise yet engaging.The story should feel authentic and raw, like a real personal anecdote shared on an online forum such as Pantip. Incorporate everyday language, emotional confessions, twists, and reflections that mirror real-life relationship struggles. Avoid overly dramatic or scripted dialogue; make it conversational and heartfelt, as if the narrator is venting or sharing their story online.Key Elements to IncludeRomantic Theme: Focus on a bittersweet romance involving themes like unexpected love, betrayal, financial hardships in relationships, jealousy, unrequited feelings, or personal growth through love.\nFirst-Person Perspective: Use \"ผม\" for a male narrator to add authenticity, sharing inner thoughts, regrets, and hopes.\nModern Thai Drama Style: Include elements like urban settings (e.g., Bangkok nightlife, apartments, workplaces), family pressures, social media influences, and emotional highs/lows typical in Thai series (e.g., love triangles, sacrifices, redemptions).\n\n",
    "context_prompt": "Use themes and characters from the highest-rated stories where the topic_detail contains ''love'' (ความรัก) in the `pantip_topics_samples` table as inspiration."
  }'
```

**Example 2:**
```sh
curl -X POST http://localhost:9090/gen/text \
  -H "Content-Type: application/json" \
  -d '{
    "db": "kratooded",
    "generation_prompt": "User Goal: Generate a Pantip-style post consisting of a title and a short, romantic story in the style of a modern Thai drama. The output must be in JSON format: {\"title\": \"...\", \"topic_detail\": \"...\"}. The topic_detail must be the story in Thai language (ภาษาไทย) only, told from a first-person perspective using \"ผม\" (male) or \"เรา\" (female) to make it feel personal and intimate. Aim for 400-600 characters in the topic_detail to keep it concise yet engaging.\n\nThe story should feel authentic and raw, like a real personal anecdote shared on an online forum such as Pantip. Incorporate everyday casual language, Thai slang (e.g., ''อะ'', ''ว่ะ'', ''เลยอะ''), emojis (e.g., 😭, 😂, 🥺), emotional confessions, twists, and reflections that mirror real-life relationship struggles. Avoid overly dramatic or scripted dialogue; make it conversational and heartfelt, as if the narrator is venting or sharing their story online. Focus on one main theme to keep it coherent, such as unexpected love leading to personal growth despite financial hardships, with a bittersweet ending that includes hope or reflection. End the topic_detail with 1-2 open-ended questions or choices to encourage comments and engagement, like asking for opinions or similar experiences but not too direct asking (just saying and open for discussion).\n\nKey Elements to Include:\n- Romantic Theme: Focus on a bittersweet romance involving themes like unexpected love, betrayal, financial hardships in relationships, jealousy, unrequited feelings, or personal growth through love. Ensure it is romantic with moments of tenderness amid struggles.\n- First-Person Perspective: Use \"ผม\" for a male narrator or \"เรา\" for a female narrator to add authenticity, sharing inner thoughts, regrets, and hopes.\n- Modern Thai Drama Style: Include elements like urban settings (e.g., Bangkok nightlife, apartments, workplaces), family pressures, social media influences, and emotional highs/lows typical in Thai series (e.g., love triangles, sacrifices, redemptions). Do not list too many problems; focus on 1-2 key conflicts for depth.\n\nEmphasize creating a focused narrative with romantic elements, drawing from real-life anecdotes like unexpected encounters in nightlife leading to deep connections, financial struggles testing love, and personal reflections on growth. Make the title dramatic and engaging to attract clicks.\n\nOutput exactly in the specified JSON format, with no additional text.\n\n",
    "context_prompt": "Use highest-rated stories where the topic_detail contains ''love'' (ความรัก) in the `pantip_topics_samples` table as inspiration."
  }'
```

### Data Pipeline API

These endpoints allow you to manage the data lifecycle, from ingesting remote data to building the local knowledge graph that powers advanced generation.

#### `POST /ingest/firebase`

Triggers a server-side dump of a Firestore collection into the corresponding project's local SQLite database.

**Request Body:** `{"project_id": "...", "collection": "...", ...}`

**Example:**
```sh
curl -X POST http://localhost:9090/ingest/firebase \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "project_id": "kratooded",
    "collection": "pantip_topics_samples",
    "incremental": true,
    "timestamp_field": "created_time",
    "limit": 100
  }'
```

#### `POST /graph/build`

Builds or updates the in-memory Knowledge Graph from a specified table in a project's local SQLite database.

**Request Body:** `{"db": "...", "table_name": "..."}`

**Example:**
```sh
curl -X POST http://localhost:9090/graph/build \
  -H "Content-Type: application/json" \
  -d '{
    "db": "kratooded",
    "table_name": "pantip_topics_samples"
  }'
```
