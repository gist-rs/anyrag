# `anyrag` Library

This crate provides the core logic for translating natural language prompts into BigQuery SQL queries and executing them. It leverages Google's Gemini API for the natural language processing and integrates directly with the Google BigQuery API for query execution.

This library is the foundation of the `anyrag` workspace and is used by the `anyrag-server` crate to expose its functionality over a REST API.

## Features

*   **Natural Language Processing:** Converts plain English prompts into executable BigQuery SQL queries.
*   **Direct BigQuery Integration:** Seamlessly executes generated SQL on your specified BigQuery project.
*   **Robust and Asynchronous:** Built with Tokio for efficient, non-blocking I/O.

## Prerequisites

Before using this library, ensure you have the following:

1.  **Rust:** The Rust programming language and Cargo. You can install it from [rustup.rs](https://rustup.rs/).
2.  **Google Cloud Account:** An active Google Cloud account with a BigQuery project set up.
3.  **Gemini API Key:** An API key for the Google Gemini API. You can obtain one from the [Google AI Studio](https://aistudio.google.com/app/apikey).
4.  **GCP Authentication:** For local development, you must be authenticated with the Google Cloud SDK. Run the following command and follow the instructions:
    ```sh
    gcloud auth application-default login
    ```

## IAM Permissions

The service account or user running the application needs the following IAM roles on your BigQuery project:

*   `roles/bigquery.dataViewer`: To inspect table schemas.
*   `roles/bigquery.jobUser`: To execute SQL queries.

You can grant these roles using the `gcloud` CLI.

## Configuration

The library is configured using environment variables. You can create a `.env` file in the root of the workspace or in this crate's directory (`crates/lib`).

**Required Environment Variables:**

*   `AI_API_KEY`: Your API key for the chosen AI provider. Required for the default `gemini` provider.
*   `AI_API_URL`: The full API endpoint URL for the AI provider.
*   `BIGQUERY_PROJECT_ID`: The ID of your Google Cloud project where BigQuery is enabled.

**Optional Environment Variables:**

*   `AI_PROVIDER`: The AI provider to use. Can be "gemini" or "local". Defaults to `gemini`.
*   `AI_MODEL`: The specific model name to use, which is mainly for the `local` provider.
*   `RUST_LOG`: Sets the logging level for tracing. For example, `RUST_LOG=info,anyrag=debug`.

## Usage

To use this library in your own project, you would typically add it as a dependency in your `Cargo.toml`. For programmatic usage examples, please refer to the `examples` directory.

## Running Tests

You can run the tests for this specific crate from the workspace root:

```sh
cargo test -p anyrag
```

## Core Dependencies

This library relies on several key Rust crates:

*   [**`tokio`**](https://crates.io/crates/tokio): For the asynchronous runtime.
*   [**`reqwest`**](https://crates.io/crates/reqwest): For making HTTP requests to the Gemini API.
*   [**`gcp-bigquery-client`**](https://crates.io/crates/gcp-bigquery-client): For interacting with the Google BigQuery API.
*   [**`serde`**](https://crates.io/crates/serde): For serializing and deserializing data.
*   [**`tracing`**](https://crates.io/crates/tracing): For application-level logging.