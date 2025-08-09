# `anyrag` Library

This crate provides the core logic for translating natural language prompts into executable queries (e.g., SQL) and executing them. It leverages a configurable AI provider for natural language processing and integrates with different storage backends like Google BigQuery.

This library is the foundation of the `anyrag` workspace and is used by the `anyrag-server` crate to expose its functionality over a REST API.

## Features

*   **Natural Language to Query:** Converts plain English prompts into executable queries (e.g., SQL).
*   **Pluggable Providers:** Supports different AI and storage providers (defaulting to Gemini and BigQuery).
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

### Advanced Usage: Full Customization

For more advanced scenarios, the `execute_prompt_with_options` method provides complete control over the prompt execution pipeline. By using the `ExecutePromptOptions` struct, you can override the default prompts for both query generation and response formatting.

This is particularly useful for:
*   Adapting to different database schemas or query languages.
*   Changing the AI's persona (e.g., making it a translator instead of a query expert).
*   Customizing the final output format beyond simple instructions.

#### Overriding Prompts

-   `system_prompt_template`: Bypasses the default query generation logic entirely. The AI will use this system prompt and your user prompt directly. This is ideal for generic, non-query tasks.
-   `user_prompt_template`: Modifies the default prompt for query generation. Use placeholders like `{prompt}`, `{context}`, and `{language}`.
-   `format_system_prompt_template`: Overrides the system prompt used in the final response formatting step.

#### Example

Here's how you could use these options to make the AI act as a cheerful assistant who formats the final response with a winking face:

```/dev/null/example.rs
use anyrag::{ExecutePromptOptions, PromptClient, PromptClientBuilder};
// ... setup client ...

let options = ExecutePromptOptions {
    prompt: "What is the total word_count for the corpus 'kinghenryv'?".to_string(),
    table_name: Some("bigquery-public-data.samples.shakespeare".to_string()),
    instruction: Some("Answer with only the number.".to_string()),
    format_system_prompt_template: Some(
        "You are a cheerful AI. You always add a winking face ;) at the end.".to_string(),
    ),
    ..Default::default()
};

let result = client.execute_prompt_with_options(options).await?;
// result will contain something like "27894 ;)"
```

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