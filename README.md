# AnyRag: Self-Improving Knowledge Base & RAG Engine

A Rust-based platform for building a self-improving knowledge base from GitHub sources — code examples, documentation, Rust docs — using natural language.

## Core Features

- **Multi-Source Ingestion** — Build a knowledge base from diverse sources (GitHub-primary):
  - **GitHub repositories** — clone repos, extract code examples/tests/src, version-aware search with embeddings
  - Web URLs (Raw HTML or Jina Reader API)
  - PDF documents (file upload or URL)
  - RSS Feeds
  - Google Sheets (generic tables or Q&A pairs)
  - Firebase Firestore collections
  - Raw Text (auto-chunked)
  - Notion Databases *(standalone crate)*
  - Local Markdown files *(standalone crate)*
- **AI-Powered Distillation** — Uses an LLM to automatically extract structured Q&A pairs and generate new ones from unstructured text, restructured into YAML sections.
- **Vector Embeddings** — Generates embeddings for semantic search across all ingested content.
- **Advanced RAG Pipeline** — Multi-stage hybrid search with LLM query analysis, parallel retrieval (metadata + vector + keyword), and Reciprocal Rank Fusion re-ranking.
- **Temporal Reasoning** — Understands time-sensitive queries like "what is the newest..." by filtering results based on date properties.
- **Knowledge Graph** — In-memory or RocksDB-backed graph with time-based validity for fact retrieval.
- **Text-to-SQL** — Translates natural language prompts into executable SQL queries for Google BigQuery or local SQLite.
- **Code RAG** — Ingest and search code examples from public GitHub repositories.
- **Raven Routed Slot Memory** — Deterministic slot-based memory for code RAG. Documents are routed to named slots (e.g., `architecture`, `types`, `apis`, `dependencies`, `tests`, `chatter`) via keyword matching. Frozen slots never decay; non-frozen slots decay over time following Raven Equation 18: `score(t) = score₀ × exp(-λΔt)`. Slot-filtered search reduces context pollution by retrieving only from active slots.
- **Episodic Memory & Self-Improving Cycle** — Record translation episodes, verify compilation results, and drive a state machine (`Collecting` → `Synthesizing` → `Exporting` → `Training` → `Upgrading`) toward autonomous JSONL export. Export FAQ and episodes as structured JSONL for downstream training pipelines.
- **Identity & Ownership** — JWT + Google OAuth2 authentication with deterministic "Guest User" fallback. Search results are filtered by owner.
- **Config-Driven** — YAML configuration with environment variable substitution, per-provider prompt templates, and `prompt.yml` overrides.

## The Advanced RAG Pipeline

```
User Query
    │
    ▼
┌──────────────────────┐
│ 1. Query Analysis    │  LLM extracts entities + keyphrases
│    (LLM Call #1)     │
└──────────┬───────────┘
           │
           ▼
┌──────────────────────────────────────┐
│ 2. Parallel Candidate Retrieval      │
│  ┌─────────┐ ┌────────┐ ┌─────────┐ │
│  │Metadata │ │ Vector │ │Keyword  │ │
│  │ Search  │ │ Search │ │ Search  │ │
│  └────┬────┘ └───┬────┘ └────┬────┘ │
└───────┼──────────┼──────────┼───────┘
        │          │          │
        ▼          ▼          ▼
┌──────────────────────────────────────┐
│ 3. Reciprocal Rank Fusion (RRF)     │  Combine + re-rank into single list
└──────────────────┬───────────────────┘
                   │
                   ▼
┌──────────────────────────────────────┐
│ 4. Contextual Chunking              │  Parse YAML sections as focused chunks
└──────────────────┬───────────────────┘
                   │
                   ▼
┌──────────────────────────────────────┐
│ 5. Answer Synthesis (LLM Call #2)   │  Generate answer from structured context
└──────────────────────────────────────┘
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     anyrag-server                        │
│              (axum REST API, config-driven)              │
│  auth · handlers · router · state · config · types       │
└──────────────────────────┬──────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                      anyrag (lib)                        │
│          (core business logic, orchestration)            │
│  executor · search · rerank · curator · graph · slots    │
│  prompts · providers · ingest · types · constants        │
└────────┬───────────────────────────────────┬────────────┘
         │                                   │
    ┌────┴─────┐                       ┌─────┴──────┐
    │  AI      │                       │  DB        │
    │Providers │                       │Providers   │
    │(local,   │                       │(SQLite,    │
    │ gemini)  │                       │ BigQuery)  │
    └──────────┘                       └────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────┐
│                   Ingestion Plugins                      │
│  (each implements the `Ingestor` trait)                  │
│                                                          │
│  web · pdf · rss · sheets · text · notion · firebase    │
│  github · markdown                                       │
└─────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────┐
│                   Utility Crates                         │
│  html · core-access · test-utils                         │
└─────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────┐
│                   Application Crates                     │
│  cli · gof                                               │
└─────────────────────────────────────────────────────────┘
```

### Server vs. Library Responsibility

- **`anyrag-server`**: HTTP routes, request parsing/validation, JWT + Google OAuth2 auth, response formatting. Zero business logic.
- **`anyrag` (lib)**: All business logic for RAG pipelines, ingestion orchestration, search, re-ranking, AI provider abstraction, storage provider abstraction.

### Plugin-Based Ingestion

Each ingestion crate implements the `Ingestor` trait from `anyrag-lib`:

```rust
#[async_trait]
pub trait Ingestor: Send + Sync {
    async fn ingest(
        &self,
        source: &str,
        owner_id: Option<&str>,
    ) -> Result<IngestionResult, IngestError>;
}
```

Feature flags in `anyrag-server` control which ingestion plugins are compiled:

```toml
[features]
default = ["full"]
full = ["bigquery", "graph_db", "rss", "firebase", "github", "web", "pdf", "sheets", "text"]
# Note: notion and markdown are standalone crates, not server plugins
```

## Workspace Crates

| Crate | Description |
|---|---|
| **[`anyrag`](crates/lib)** | Core library — AI/DB providers, search pipeline, re-ranking, curator, knowledge graph, episodic memory, self-improving cycle, Raven routed slot memory, ingestion traits, prompt templates, types |
| **[`anyrag-server`](crates/server)** | Axum web server — REST API with feature-flagged routes, JWT/OAuth2 auth, episodes & cycle endpoints, config-driven prompt management |
| **[`anyrag-cli`](crates/cli)** | CLI tool — `login`, `dump firebase`, `dump github`, `process`, `list`, `count` commands |
| **[`anyrag-github`](crates/github)** | GitHub ingestion — clone repos, extract code examples/tests/src, version-aware search with embeddings |
| **[`anyrag-web`](crates/web)** | Web ingestion — `WebIngestor` with `WebIngestStrategy` (`RawHtml` or `Jina`), fetch URLs, convert HTML to Markdown, AI restructuring into structured YAML |
| **[`anyrag-pdf`](crates/pdf)** | PDF ingestion — extract text from PDFs (file upload or URL), AI restructuring into structured YAML |
| **[`anyrag-rss`](crates/rss)** | RSS ingestion — parse RSS feeds, store each item as a separate document |
| **[`anyrag-sheets`](crates/sheets)** | Google Sheets ingestion — fetch public sheets as CSV, support generic tables and Q&A pairs |
| **[`anyrag-text`](crates/text)** | Text ingestion — auto-chunk raw text with overlap, store chunks as documents |
| **[`anyrag-firebase`](crates/firebase)** | Firebase ingestion — dump Firestore collections into local SQLite |
| **[`anyrag-notion`](crates/notion)** | Notion ingestion — fetch Notion database pages via API, flatten properties to text *(standalone, not a server plugin)* |
| **[`anyrag-markdown`](crates/markdown)** | Markdown ingestion — split local `.md` files by separator, optional embedding generation *(standalone, not a server plugin)* |
| **[`anyrag-html`](crates/html)** | HTML utilities — clean HTML tags, convert to Markdown, fetch URLs to cleaned Markdown |
| **[`core-access`](crates/core-access)** | Identity & auth — user management with deterministic UUIDv5 IDs, role-based access (`root`/`user`/`guest`) |
| **[`gof`](crates/gof)** | Project-aware RAG CLI — auto-ingest code examples from `Cargo.toml` dependencies via crates.io resolution, MCP search protocol |
| **[`anyrag-test-utils`](crates/test-utils)** | Test utilities — in-memory DB setup, mock AI provider with FIFO response queue, PDF generation helpers |

## Project Structure

```
anyrag/
├── Cargo.toml              # Workspace configuration (16 crates)
├── EXAMPLES.md             # Detailed API usage examples
├── crates/
│   ├── lib/                # Core business logic library
│   │   └── src/
│   │       ├── executor.rs     # High-level orchestrator
│   │       ├── search.rs       # Multi-stage hybrid search
│   │       ├── rerank.rs       # RRF + LLM re-ranking
│   │       ├── curator.rs      # Automated knowledge synthesis
│   │       ├── cycle.rs        # Self-improving cycle state machine
│   │       ├── graph/          # Knowledge graph (indradb)
│   │       ├── prompts/        # System/user prompt templates
│   │       ├── providers/      # AI + DB provider abstractions
│   │       ├── slots/          # Raven Routed Slot Memory (types, router, decay, search, ingest, seeder)
│   │       ├── ingest/         # Ingestion traits, episodic memory, knowledge, embeddings
│   │       └── types.rs        # Shared data structures
│   ├── server/             # Axum REST API server
│   │   └── src/
│   │       ├── router.rs       # Feature-flagged route definitions
│   │       ├── config.rs       # YAML config with env var substitution
│   │       ├── state.rs        # AppState, AI/DB providers, cycle mutex
│   │       ├── auth/           # JWT + Google OAuth2
│   │       └── handlers/       # Route handlers (ingest, search, admin, episodes)
│   ├── cli/                # Administrative CLI
│   ├── github/             # GitHub repo ingestion + code RAG
│   ├── web/                # Web URL ingestion
│   ├── pdf/                # PDF ingestion
│   ├── rss/                # RSS feed ingestion
│   ├── sheets/             # Google Sheets ingestion
│   ├── text/               # Raw text ingestion
│   ├── notion/             # Notion database ingestion
│   ├── firebase/           # Firestore collection ingestion
│   ├── markdown/           # Local Markdown file ingestion
│   ├── html/               # HTML → Markdown utilities
│   ├── core-access/        # Identity & authN/authZ
│   ├── gof/                # Project-aware dependency RAG CLI
│   └── test-utils/         # Shared test infrastructure
└── deploy.sh               # Google Cloud Run deployment script
```

## API Response Structure

All JSON responses follow a consistent `result` object structure. Append `?debug=true` for contextual debug info.

**Standard:**
```json
{
  "result": {
    "message": "Ingestion successful",
    "ingested_articles": 2
  }
}
```

**Debug:**
```json
{
  "debug": { "url": "http://example.com/rss" },
  "result": {
    "message": "Ingestion successful",
    "ingested_articles": 2
  }
}
```

## API Endpoints

### Ingestion

| Method | Path | Feature Flag | Description |
|---|---|---|---|
| `POST` | `/ingest/web` | `web` | Fetch and process a web URL |
| `POST` | `/ingest/pdf` | `pdf` | Process PDF (upload or URL) |
| `POST` | `/ingest/rss` | `rss` | Ingest articles from RSS feed |
| `POST` | `/ingest/sheet` | `sheets` | Ingest Google Sheet data |
| `POST` | `/ingest/text` | `text` | Ingest raw text (auto-chunked) |
| `POST` | `/ingest/github` | `github` | Ingest GitHub repo code examples |
| `POST` | `/ingest/firebase` | `firebase` | Dump Firestore to SQLite |
| `GET`  | `/examples/{repo}` | `github` | Get extracted examples (latest version) |
| `GET`  | `/examples/{repo}/{ver}` | `github` | Get extracted examples (specific version) |

### Search & RAG

| Method | Path | Feature Flag | Description |
|---|---|---|---|
| `POST` | `/search/knowledge` | — | **Primary RAG endpoint** — hybrid search + synthesis |
| `POST` | `/search/examples` | `github` | **Code RAG** — search GitHub code examples |
| `POST` | `/search/hybrid` | — | Hybrid search (vector + keyword) with re-ranking |
| `POST` | `/search/vector` | — | Pure vector similarity search |
| `POST` | `/search/keyword` | — | Pure keyword search |
| `POST` | `/search/knowledge_graph` | `graph_db` | Graph fact lookup |

### Generation & Admin

| Method | Path | Description |
|---|---|---|
| `POST` | `/prompt` | Natural language → SQL → formatted result |
| `POST` | `/db/query` | Execute raw read-only SQL |
| `POST` | `/gen/text` | Two-step generation (context retrieval → synthesis) |
| `POST` | `/embed/new` | Generate embeddings for unembedded docs |
| `GET`  | `/knowledge/export` | Export FAQ as JSONL for fine-tuning |
| `POST` | `/graph/build` | Build knowledge graph from table (`graph_db`) |
| `GET`  | `/documents` | List visible documents |
| `GET`  | `/users` | List users (admin only) |

### Episodes & Self-Improving Cycle

| Method | Path | Description |
|---|---|---|
| `POST` | `/episodes` | Record a new translation episode |
| `GET`  | `/episodes` | List recorded episodes (query: `limit`, `since`, `successful_only`) |
| `GET`  | `/episodes/stats` | Get episodic memory statistics |
| `POST` | `/episodes/{id}/verify` | Verify compilation result for an episode |
| `GET`  | `/cycle/status` | Get current cycle state machine status |
| `POST` | `/cycle/trigger` | Trigger a cycle tick to potentially advance state |

### Slot Management & Routed Search

| Method | Path | Description |
|---|---|---|
| `GET` | `/slots` | List all slots with document counts (auto-seeds defaults if empty) |
| `POST` | `/slots` | Create a custom slot with keywords and decay rate |
| `POST` | `/search/slots` | Slot-filtered search with decay scoring |
| `GET` | `/slots/{name}/documents` | List documents in a slot with decayed scores |
| `DELETE` | `/slots/{name}/documents/{doc_id}` | Remove a document from a slot |
| `POST` | `/slots/reindex` | Re-route all documents through keyword router |

**Default Slots (auto-seeded):**

| Slot | Frozen | Decay Rate (λ) | Purpose |
|---|---|---|---|
| `architecture` | ✅ Yes | 0.0 | System design, module structure, high-level patterns |
| `types` | No | 0.05 | Type definitions, structs, enums, type aliases |
| `apis` | No | 0.05 | Public API surfaces, function signatures, trait definitions |
| `dependencies` | No | 0.1 | Crate dependencies, version constraints, feature flags |
| `tests` | No | 0.1 | Test files, test utilities, benchmark harnesses |
| `chatter` | No | 0.5 | Conversational context, chat logs, informal notes |

**Example — Slot-filtered search:**
```sh
curl -X POST http://localhost:3000/search/slots \
  -H 'Content-Type: application/json' \
  -d '{"active_slots":["apis","types"],"include_frozen":true,"limit":10}'
```

### Auth

| Method | Path | Description |
|---|---|---|
| `GET` | `/auth/login/google` | Start Google OAuth2 flow |
| `GET` | `/auth/callback/google` | OAuth2 callback |
| `GET` | `/auth/me` | Get current user info |

See **[EXAMPLES.md](EXAMPLES.md)** for detailed `curl` examples for every endpoint.

## Configuration

The server uses a layered YAML config system with environment variable substitution:

```
config.yml              # Main config (required)
prompt.yml              # User prompt overrides (optional)
config.local.yml        # Provider-specific template (fallback)
```

**Layer order** (later layers override earlier):
1. Programmatic defaults (built-in prompt templates)
2. `config.yml` (with `${ENV_VAR}` substitution)
3. `prompt.yml` (with `${ENV_VAR}` substitution)
4. Environment variables (`PORT`, `DB_URL`)
5. Prefixed env vars (`ANYRAG_EMBEDDING__API_URL`)

Key environment variables:

| Variable | Description |
|---|---|
| `AI_API_KEY` | LLM API key |
| `AI_API_URL` | LLM API base URL |
| `AI_MODEL` | LLM model name |
| `AI_PROVIDER` | Provider template (`local` or `gemini`) |
| `EMBEDDINGS_API_URL` | Embedding API URL |
| `EMBEDDINGS_MODEL` | Embedding model name |
| `JINA_API_KEY` | Jina Reader API key (for web ingestion) |
| `PORT` | Server port (default: `9090`) |

## Getting Started

### Build & Run

```sh
# Build all crates
cargo build --workspace

# Run server (requires config.yml in crates/server/)
cargo run --bin server

# Run tests
cargo test --workspace

# Run with clean logs
RUST_LOG=info cargo run --bin server --quiet
```

### CLI Usage

```sh
# Login via Google OAuth2
cargo run --bin cli -- login

# Dump Firestore collection to local SQLite
cargo run --bin cli -- dump firebase --project-id my-project --collection my-collection

# Dump GitHub repo code examples
cargo run --bin cli -- dump github --url https://github.com/user/repo

# List rows from local database
cargo run --bin cli -- list my_table --project-id my-project

# Count rows
cargo run --bin cli -- count my_table --project-id my-project

# Export training data as JSONL (for LoRA fine-tuning)
cargo run --bin cli -- export --max-episodes 1000 --output output/training.jsonl
```

### GoF (Project-Aware RAG CLI)

```sh
# Auto-ingest examples from all Cargo.toml dependencies
cargo run --bin gof -- example --path ./Cargo.toml

# Ingest all content types (examples, tests, source)
cargo run --bin gof -- example --all

# Search ingested code examples (MCP protocol)
cargo run --bin gof -- mcp "how to connect to database" --repos user-repo
```

## Deployment to Google Cloud Run

### Prerequisites

- [Google Cloud SDK](https://cloud.google.com/sdk/docs/install) installed and initialized
- Google Cloud project with billing enabled
- `crates/server/.env` with `AI_API_KEY` and `BIGQUERY_PROJECT_ID`

### Deploy

```sh
chmod +x deploy.sh
./deploy.sh your-gcp-project-id
```

The script handles service enablement, secret management, service accounts, and Cloud Build.

## Fundamental Data Strategy: Structured Contextual Chunking

Documents are broken into **structured YAML chunks** — each section stored as an independent document. This enables:

- **Focused Retrieval** — The RAG pipeline finds the exact section answering the user's question, not the whole document.
- **Accuracy** — The LLM receives precisely relevant context, reducing hallucination.
- **Efficiency** — Smaller contexts mean lower latency and cost.
- **Consistency** — All ingestion sources produce the same YAML-based data model.

Example ingested structure:

```yaml
sections:
  - title: "Eligibility Requirements"
    content: "Must be 18+ with valid ID..."
    faqs:
      - question: "Who can participate?"
        answer: "Anyone 18 or older with valid identification."
```

## Running Tests

```sh
# All tests
cargo test --workspace

# Specific test
cargo test -p anyrag-server --test server_test

# With lint fixes
cargo clippy --fix --allow-dirty
```

## License

MIT