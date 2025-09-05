# `anyrag-cli`

The `anyrag-cli` is a powerful command-line interface for interacting with the `anyrag` ecosystem. It allows you to authenticate, dump data from remote sources like Google Firestore, and process it for use in RAG pipelines.

## Prerequisites

Before using the CLI, please ensure you have the following set up:

1.  **Rust**: The Rust toolchain is required to build and run the CLI. You can install it from [rustup.rs](https://rustup.rs/).
2.  **Authentication**: You need a way to authenticate with Google Cloud. You have two options:
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

Initiates a browser-based OAuth2 flow to authenticate the CLI with your Google account. This is required if you are not using a service account and need to access protected resources. The resulting token is stored securely in your operating system's keychain.

**Usage:**
```sh
cargo run -p cli -- login
```
Upon success, you will see a `✅ Login successful!` message.

### `dump firebase`

This is the primary command for fetching data from a Google Firestore collection and storing it in a local SQLite database (`db/anyrag.db`). It supports both full and incremental dumps, making it efficient for keeping your local data in sync.

**Arguments:**

*   `--project-id <PROJECT_ID>`: (Optional) The ID of your Google Cloud project. If omitted, the CLI will automatically use the `project_id` from your `gcp_creds.json` file.
*   `--collection <COLLECTION_NAME>`: **(Required)** The name of the Firestore collection you want to dump.
*   `--incremental`: (Optional) Enables incremental sync mode. When used, the CLI will only fetch documents that are newer than the last sync point.
*   `--timestamp-field <FIELD_NAME>`: (Required if `--incremental` is used) The name of the document field that contains the update/creation timestamp (e.g., `updatedAt`, `createdAt`). This field is used to determine which documents are new.
*   `--limit <NUMBER>`: (Optional) Limits the number of documents to fetch. This is very useful for testing a dump without fetching the entire collection.

**Examples:**

**1. Perform a full dump (inferring Project ID from `gcp_creds.json`):**
```sh
cargo run -p cli -- dump firebase --collection users --limit 10
```

**2. Test the dump by fetching only 10 documents (explicitly providing Project ID):**
```sh
cargo run -p cli -- dump firebase --project-id my-gcp-project --collection users --limit 10
```

**3. Perform an initial incremental dump:**
This will fetch all documents and save the latest timestamp as a starting point for the next run.
```sh
cargo run -p cli -- dump firebase \
  --project-id my-gcp-project \
  --collection posts \
  --incremental \
  --timestamp-field updatedAt
```

**4. Perform a subsequent incremental dump:**
This will only fetch documents where `updatedAt` is newer than the last one saved.
```sh
cargo run -p cli -- dump firebase \
  --project-id my-gcp-project \
  --collection posts \
  --incremental \
  --timestamp-field updatedAt
```

### `process`

*(This command is planned but not yet implemented).*

This command will be used to process and enrich the data that has been dumped into the local SQLite database, preparing it for RAG and fine-tuning.
