# `anyrag-cli`

The `anyrag-cli` is a powerful command-line interface for interacting with the `anyrag` ecosystem. It allows you to authenticate, dump data from remote sources like Google Firestore and GitHub, and process it for use in RAG pipelines.

## Prerequisites

Before using the CLI, please ensure you have the following set up:

1.  **Rust**: The Rust toolchain is required to build and run the CLI. You can install it from [rustup.rs](https://rustup.rs/).
2.  **Git**: The `git` command-line tool must be installed and available in your system's PATH. This is required for the `dump github` command.
3.  **Authentication (for Firestore)**: If you plan to use the `dump firebase` command, you need a way to authenticate with Google Cloud. You have two options:
    *   **Service Account (Recommended for CI/CD)**: Place a `gcp_creds.json` service account file in the `anyrag` workspace root. The CLI will automatically use this file if it exists.
    *   **Application Default Credentials (Recommended for Local Development)**: If `gcp_creds.json` is not found, the CLI will fall back to using your local ADC. Run `gcloud auth application-default login` in your terminal to set this up.

## Running the CLI

You can run the CLI from the workspace root (`anyrag/`) using `cargo`. The general format is:

```sh
cargo run -p cli -- <COMMAND> [OPTIONS]
```

Note the `--` which separates Cargo's arguments from the CLI's arguments.

---

## Commands

The CLI is structured into several commands. You can get help for any command by adding `--help`.

### `login`

Initiates a browser-based OAuth2 flow to authenticate the CLI with your Google account. This is required if you are not using a service account and need to access protected resources on the `anyrag-server`. The resulting token is stored securely in your operating system's keychain.

**Usage:**
```sh
cargo run -p cli -- login
```
Upon success, you will see a `✅ Login successful!` message.

### `dump`

This command is used to fetch data from various remote sources and store it locally for analysis and RAG.

#### `dump firebase`

Fetches data from a Google Firestore collection and stores it in a local SQLite database (`db/<project_id>.db`). It supports both full and incremental dumps, making it efficient for keeping your local data in sync.

**Arguments:**

*   `--project-id <PROJECT_ID>`: (Optional) The ID of your Google Cloud project. If omitted, the CLI will automatically use the `project_id` from your `gcp_creds.json` file.
*   `--collection <COLLECTION_NAME>`: **(Required)** The name of the Firestore collection you want to dump.
*   `--incremental`: (Optional) Enables incremental sync mode. When used, the CLI will only fetch documents that are newer than the last sync point.
*   `--timestamp-field <FIELD_NAME>`: (Required if `--incremental` is used) The name of the document field that contains the update/creation timestamp (e.g., `updatedAt`).
*   `--limit <NUMBER>`: (Optional) Limits the number of documents to fetch.
*   `--fields <FIELDS>`: (Optional) A comma-separated list of specific fields to select from the documents (e.g., `title,author,rating`).

**Examples:**

**1. Perform a full dump:**
```sh
cargo run -p cli -- dump firebase --collection users --limit 10
```

**2. Perform an incremental dump:**
```sh
cargo run -p cli -- dump firebase \
  --project-id my-gcp-project \
  --collection posts \
  --incremental \
  --timestamp-field updatedAt
```

#### `dump github`

Clones a public GitHub repository, extracts all Rust code examples (from `README.md`, `examples/`, `tests/`, and doc comments), and stores them in a versioned, repository-specific SQLite database (`db/github_ingest/<repo_name>.db`). This creates a searchable knowledge base of your code examples.

**Arguments:**

*   `--url <URL>`: **(Required)** The URL of the public GitHub repository to ingest.
*   `--version <VERSION>`: (Optional) The specific Git tag or branch to check out (e.g., `v0.1.0`). If omitted, the version will be inferred from the repository's `Cargo.toml` file.
*   `--embedding-api-url <URL>`: (Optional) The API endpoint for a text embedding model. If provided, embeddings will be generated for each extracted code example.
*   `--embedding-model <MODEL_NAME>`: (Required if `--embedding-api-url` is set) The name of the embedding model to use.

**Example:**

This command ingests version `v1.2.3` of the `tokio` repository and generates embeddings for all found examples.
```sh
cargo run -p cli -- dump github \
  --url https://github.com/tokio-rs/tokio \
  --version v1.2.3 \
  --embedding-api-url "http://localhost:1234/api/embeddings" \
  --embedding-model "text-embedding-qwen3-embedding-8b"
```

### `process file`

Ingests a local Markdown file by splitting it into chunks and storing them in a dedicated SQLite database. This is the same logic that `dump github` uses automatically on its generated context file, but it can be used on any Markdown file.

**Arguments:**

*   `<FILE_PATH>`: **(Required)** The path to the local Markdown file to process.
*   `--db-path <DB_PATH>`: **(Required)** The path where the output SQLite database will be created.
*   `--separator <SEPARATOR>`: (Optional) The string used to split the file into chunks. Defaults to `"\n---\n"`.
*   `--embedding-api-url <URL>`: (Optional) The API endpoint for a text embedding model. If provided, embeddings will be generated for each chunk.
*   `--embedding-model <MODEL_NAME>`: (Required if `--embedding-api-url` is set) The name of the embedding model to use.

**Example:**

This command will take a local context file, split it by the `---` separator, and store each chunk in a new database file named `my-project.db`, generating embeddings for each chunk.
```sh
cargo run -p cli -- process file my-project-context.md \
  --db-path db/chunks/my-project.db \
  --separator "---" \
  --embedding-api-url "http://localhost:1234/api/embeddings" \
  --embedding-model "text-embedding-qwen3-embedding-8b"
```
