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

Clones a public GitHub repository, intelligently extracts code examples from documentation, tests, and example files, stores them in a local database, and generates a consolidated Markdown file for use as LLM context.

**Arguments:**

*   `<URL>`: **(Required)** The full URL of the public GitHub repository (e.g., `https://github.com/tursodatabase/turso`).
*   `--version <VERSION>`: (Optional) A specific git tag, branch, or commit hash to check out. If omitted, the CLI will automatically use the latest semantic version tag it finds.

**Example:**

This command will ingest the Turso repository, find all relevant examples, and generate a context file.

```sh
cargo run -p cli -- dump github https://github.com/tursodatabase/turso
```

**Expected Output:**

After a successful run, you will see messages indicating the number of examples ingested, and a new file will be created in your current directory named after the repository, such as `tursodatabase-turso-context.md`. This file contains all the extracted examples, formatted and ready to be used as a context file for an LLM.

### `process`

*(This command is planned but not yet implemented).*

This command will be used to process and enrich the data that has been dumped into the local SQLite database, preparing it for RAG and fine-tuning.