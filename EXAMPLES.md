# API Usage Examples

Detailed `curl` examples for every `anyrag-server` API endpoint.

## Base URL

All examples assume the server is running locally:

```
http://localhost:9090
```

## Authentication

Most endpoints accept an optional `Authorization` header. Without it, requests are processed as the **Guest User**.

```sh
# Example with JWT authentication
curl -X POST http://localhost:9090/some/endpoint \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{...}'
```

---

## Health & Root

### `GET /`

Root endpoint. Returns a basic welcome message.

```sh
curl http://localhost:9090/
```

### `GET /health`

Health check endpoint.

```sh
curl http://localhost:9090/health
```

---

## Authentication API

### `GET /auth/login/google`

Initiates the Google OAuth2 login flow. Redirects the user to Google's consent screen.

```sh
# Open in browser or follow redirect
curl -v http://localhost:9090/auth/login/google
```

### `GET /auth/callback/google`

OAuth2 callback endpoint. Google redirects here after user consent. Exchanges the authorization code for user info and returns a JWT.

```sh
# Typically called by Google's redirect, not manually
curl "http://localhost:9090/auth/callback/google?code=<auth_code>&state=<state>"
```

### `GET /auth/me`

Returns the current authenticated user's info.

```sh
curl http://localhost:9090/auth/me \
  -H "Authorization: Bearer <your_jwt>"
```

**Example Response:**
```json
{
  "result": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "user@example.com",
    "role": "user"
  }
}
```

---

## Knowledge Base Ingestion API

### `POST /ingest/web` *(feature: `web`)*

Fetches and processes content from a web URL.

**Query Parameters:**
- `faq` (boolean, optional): If `true`, runs the full AI pipeline to distill content into structured Q&A pairs. Defaults to `false`.
- `embed` (boolean, optional): If `true` (default), generates vector embeddings.

**Request Body:** `{"url": "https://..."}`

**Example — Light Ingest (store content only):**
```sh
curl -X POST http://localhost:9090/ingest/web \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "url": "https://www.true.th/betterliv/support/true-app-mega-campaign"
  }'
```

**Example — Full FAQ Generation:**
```sh
curl -X POST "http://localhost:9090/ingest/web?faq=true" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "url": "https://www.true.th/betterliv/support/true-app-mega-campaign"
  }'
```

---

### `POST /ingest/pdf` *(feature: `pdf`)*

Processes a PDF from a file upload or URL. Supports up to 10MB uploads.

**Query Parameters:**
- `faq` (boolean, optional): If `true` (default), runs the full AI pipeline.
- `embed` (boolean, optional): If `true` (default), generates vector embeddings.

**Request Body:** `multipart/form-data` with either a `file` or `url` field.
- `extractor`: (optional) `"local"` (default) or `"gemini"`.

**Example — File Upload:**
```sh
curl -X POST "http://localhost:9090/ingest/pdf?faq=true" \
  -H "Authorization: Bearer <your_jwt>" \
  -F "file=@/path/to/your/document.pdf" \
  -F "extractor=local"
```

**Example — From URL:**
```sh
curl -X POST "http://localhost:9090/ingest/pdf?faq=true" \
  -H "Authorization: Bearer <your_jwt>" \
  -F "url=https://arxiv.org/pdf/2403.05530.pdf" \
  -F "extractor=local"
```

---

### `POST /ingest/rss` *(feature: `rss`)*

Ingests articles from an RSS feed URL. Each item is stored as a separate document.

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

**Example Response:**
```json
{
  "result": {
    "message": "Ingestion successful",
    "ingested_articles": 2
  }
}
```

---

### `POST /ingest/sheet` *(feature: `sheets`)*

Ingests data from a public Google Sheet.

**Query Parameters:**
- `faq` (boolean, optional): If `true`, ingests a sheet with "Question" and "Answer" columns as Q&A pairs. If `false` (default), ingests as a generic table.

**Request Body:** `{"url": "...", "gid": "...", "skip_header": true}`

**Example — Generic Table:**
```sh
curl -X POST http://localhost:9090/ingest/sheet \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "url": "https://docs.google.com/spreadsheets/d/your_sheet_id/edit"
  }'
```

**Example — FAQ Ingest:**
```sh
curl -X POST "http://localhost:9090/ingest/sheet?faq=true" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "url": "https://docs.google.com/spreadsheets/d/your_sheet_id/edit",
    "gid": "856666263"
  }'
```

---

### `POST /ingest/text` *(feature: `text`)*

Ingests raw text. Text is automatically chunked with overlap.

**Query Parameters:**
- `faq` (boolean, optional): If `false` (default), the text is auto-chunked.

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

---

### `POST /ingest/firebase` *(feature: `firebase`)*

Triggers a server-side dump of a Firestore collection into local SQLite.

**Request Body:** `{"project_id": "...", "collection": "...", ...}`

**Example:**
```sh
curl -X POST http://localhost:9090/ingest/firebase \
  -H "Content-Type: application/json" \
  -d '{
    "project_id": "kratooded",
    "collection": "pantip_topics_samples"
  }'
```

---

## GitHub Code Ingestion & RAG API

### `POST /ingest/github` *(feature: `github`)*

Triggers ingestion of a public GitHub repository. The server clones the repo, extracts code examples, generates embeddings, and stores them. The response includes the ingested version.

**Request Body:** `{"url": "...", "version": "..."}` (version is optional)

**Example — Auto-detect latest version:**
```sh
curl -X POST http://localhost:9090/ingest/github \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://github.com/tursodatabase/turso"
  }'
```

**Example Response:**
```json
{
  "result": {
    "message": "GitHub ingestion pipeline completed successfully.",
    "ingested_examples": 95,
    "version": "v0.100.0"
  }
}
```

### `GET /examples/{repo_name}`

Retrieves a consolidated Markdown file of all extracted examples for the **latest ingested version**.

**Example:**
```sh
curl "http://localhost:9090/examples/tursodatabase-turso"
```

### `GET /examples/{repo_name}/{version}`

Retrieves extracted examples for a **specific version**.

**Example:**
```sh
curl "http://localhost:9090/examples/tursodatabase-turso/v0.100.0" \
  -H "Authorization: Bearer <your_jwt>"
```

---

## Search & RAG API

### `POST /search/knowledge`

**Primary RAG endpoint.** Takes a user's question, performs a multi-stage hybrid search (query analysis → parallel retrieval → RRF re-ranking → contextual chunking), and synthesizes a final answer.

**Request Body:** `{"query": "...", "limit": 5, "instruction": "..."}`

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

**Example — With database override:**
```sh
curl -X POST http://localhost:9090/search/knowledge \
  -H "Content-Type: application/json" \
  -d '{
    "db": "anyrag-thai",
    "query": "มีเงิน 2 หมื่นออมต่อได้มั้ย"
  }'
```

---

### `POST /search/examples` *(feature: `github`)*

**Code RAG endpoint.** Performs RAG search across ingested GitHub repositories to find relevant code examples.

**Request Body:** `{"query": "...", "repos": ["..."]}`

**Example:**
```sh
curl -X POST http://localhost:9090/search/examples \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "query": "how to connect to a database with rust",
    "repos": ["tursodatabase-turso"]
  }'
```

---

### `POST /search/hybrid`

Performs a hybrid search (vector + keyword) with re-ranking. Useful for testing or custom retrieval strategies.

**Request Body:** `{"query": "...", "limit": 10, "mode": "rrf"}`

**Modes:** `"rrf"` or `"llm_rerank"` (default)

**Example:**
```sh
curl -X POST http://localhost:9090/search/hybrid \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "query": "Tesla prize conditions",
    "mode": "rrf"
  }'
```

---

### `POST /search/vector`

Pure vector similarity search against the knowledge base.

**Request Body:** `{"query": "...", "limit": 10}`

**Example:**
```sh
curl -X POST http://localhost:9090/search/vector \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "query": "machine learning basics"
  }'
```

---

### `POST /search/keyword`

Pure keyword search against the knowledge base.

**Request Body:** `{"query": "...", "limit": 10}`

**Example:**
```sh
curl -X POST http://localhost:9090/search/keyword \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "query": "Tesla campaign"
  }'
```

---

### `POST /search/knowledge_graph` *(feature: `graph_db`)*

Performs a direct search on the in-memory knowledge graph for a specific fact.

**Request Body:** `{"subject": "...", "predicate": "..."}`

**Example:**
```sh
curl -X POST http://localhost:9090/search/knowledge_graph \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "subject": "Alice",
    "predicate": "role"
  }'
```

---

## Embedding & Export API

### `POST /embed/new`

Generates vector embeddings for all unembedded documents.

**Request Body:** `{"limit": 100}` (optional)

**Example:**
```sh
curl -X POST http://localhost:9090/embed/new \
  -H "Content-Type: application/json" \
  -d '{"limit": 50}'
```

### `GET /knowledge/export`

Exports the FAQ knowledge base as a JSONL file suitable for fine-tuning.

**Example:**
```sh
curl http://localhost:9090/knowledge/export -o finetuning_dataset.jsonl
```

---

## Graph API

### `POST /graph/build` *(feature: `graph_db`)*

Builds or updates the in-memory Knowledge Graph from a specified table.

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

---

## Advanced API

### `POST /prompt`

Translates a natural language prompt into a SQL query, executes it, and formats the result.

**Example — Basic Query:**
```sh
curl -X POST http://localhost:9090/prompt \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "tell me a joke"
  }'
```

**Example — Shorthand Query:**
```sh
# This shorthand is automatically translated into a full SQL query
curl -X POST http://localhost:9090/prompt \
  -H "Content-Type: application/json" \
  -d '{
    "db": "kratooded",
    "prompt": "ls pantip_topics_samples limit=20"
  }'
```

---

### `POST /db/query`

Executes a raw, read-only SQL query against a project's database.

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

---

### `POST /gen/text`

A powerful two-step generation endpoint. First runs a `context_prompt` to retrieve data, then uses that data as context for a `generation_prompt`.

**Example 1 — Structured Generation:**
```sh
curl -X POST 'http://localhost:9090/gen/text?debug=false' \
  -H "Content-Type: application/json" \
  -d '{
    "db": "kratooded",
    "model": "gemini-2.5-pro",
    "generation_prompt": "Generate a short summary of the top-rated stories.",
    "context_prompt": "Use top ten `rating` stories from the `pantip_topics_samples` table."
  }'
```

**Example 2 — Creative Generation:**
```sh
curl -X POST 'http://localhost:9090/gen/text?debug=false' \
  -H "Content-Type: application/json" \
  -d '{
    "db": "kratooded",
    "model": "gemini-2.5-pro",
    "generation_prompt": "Generate a Pantip-style post in Thai language.",
    "context_prompt": "ความรักที่ไม่สมหวังซ้ำๆ ซากๆ"
  }'
```

---

## Admin & Utility API

### `GET /documents`

Lists all documents visible to the current user. Root users see all documents; other users see their own and guest-owned documents.

**Example:**
```sh
curl http://localhost:9090/documents \
  -H "Authorization: Bearer <your_jwt>"
```

### `GET /users`

**(Admin only)** Lists all users. Requires the `root` role.

**Example:**
```sh
curl http://localhost:9090/users \
  -H "Authorization: Bearer <your_jwt_with_root_role>"
```

---

## Debug Mode

Append `?debug=true` to any request URL to include a `debug` object in the response:

**Example:**
```sh
curl "http://localhost:9090/ingest/rss?debug=true" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{"url": "http://example.com/feed.xml"}'
```

**Response:**
```json
{
  "debug": {
    "url": "http://example.com/feed.xml"
  },
  "result": {
    "message": "Ingestion successful",
    "ingested_articles": 2
  }
}