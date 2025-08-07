# AnyQuery: Natural Language to SQL

This project is a workspace containing a Rust-based toolset to translate natural language prompts into BigQuery SQL queries and execute them. It leverages Google's Gemini API for translation and integrates directly with the Google BigQuery API.

The workspace consists of two main crates:
-   **`anyquery`**: A library providing the core logic for prompt-to-SQL conversion and BigQuery interaction.
-   **`anyquery-server`**: A lightweight web server built with `axum` that exposes the library's functionality via a REST API.

## Features

*   **Natural Language Processing:** Converts plain English prompts into executable BigQuery SQL queries.
*   **Direct BigQuery Integration:** Seamlessly executes generated SQL on your specified BigQuery project.
*   **RESTful API:** The `server` crate provides an easy-to-use API for integrations.
*   **Containerized Deployment:** Includes a multi-stage `Dockerfile` for building a minimal, secure server image.
*   **Robust and Asynchronous:** Built with Tokio for efficient, non-blocking I/O.
*   **Workspace Structure:** Clear separation of concerns between the core library and the server application.

## Project Structure

```
anyquery/
├── Cargo.toml         # Workspace configuration
├── crates/
│   ├── lib/           # The core logic library
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   ├── examples/
│   │   └── tests/
│   └── server/        # The axum web server
│       ├── Cargo.toml
│       ├── Dockerfile
│       └── src/
│           ├── main.rs
│           ├── config.rs
│           └── errors.rs
└── README.md
```

## Prerequisites

Before you begin, ensure you have the following:

1.  **Rust:** The Rust programming language and Cargo. You can install it from [rustup.rs](https://rustup.rs/).
2.  **Docker:** Docker is required for building and running the containerized application.
3.  **Google Cloud Account:** An active Google Cloud account with a BigQuery project set up.
4.  **Gemini API Key:** An API key for the Google Gemini API. You can obtain one from the [Google AI Studio](https://aistudio.google.com/app/apikey).
5.  **GCP Authentication:** For local development, you must be authenticated with the Google Cloud SDK. Run the following command and follow the instructions:
    ```sh
    gcloud auth application-default login
    ```

## IAM Permissions

The service account or user running the application needs the following IAM roles on your BigQuery project:

*   **`roles/bigquery.dataViewer`**: To inspect table schemas.
*   **`roles/bigquery.jobUser`**: To execute SQL queries.

You can grant these roles using the `gcloud` CLI.

## Configuration

The server is configured using environment variables. You can create a `.env` file in the `./`, `crates/lib`, `crates/server` directory or set them in your shell.

**Required Environment Variables:**

*   `GEMINI_API_KEY`: Your API key for the Gemini API.
*   `BIGQUERY_PROJECT_ID`: The ID of your Google Cloud project where BigQuery is enabled.

**Optional Environment Variables:**

*   `GEMINI_API_URL`: The URL for the Gemini API. Defaults to a known Gemini model endpoint.
*   `RUST_LOG`: Sets the logging level. For example, `RUST_LOG=info,anyquery=debug`.
*   `PORT`: The port for the server to listen on. Defaults to `8080`.

## Running the Server Locally

1.  Navigate to the server crate:
    ```sh
    cd crates/server
    ```
2.  Set up your environment variables (e.g., create a `.env` file).
3.  Run the server:
    ```sh
    cargo run
    ```
The server will start on port `8080` by default.

## Running Tests

You can run tests from the workspace root.

*   **Run all tests** for both the library and server:
    ```sh
    cargo test --workspace
    ```
*   **Run tests for a specific crate:**
    ```sh
    # Test the library
    cargo test -p anyquery

    # Test the server
    cargo test -p anyquery-server
    ```

## Docker Deployment

A `Dockerfile` is provided in the `crates/server` directory to build a containerized version of the server.

1.  **Build the Docker image** from the workspace root:
    ```sh
    docker build -t anyquery-server -f crates/server/Dockerfile .
    ```
    This command uses the workspace root as the build context, which is necessary for the `Dockerfile` to access all required files.

2.  **Run the Docker container:**
    ```sh
    docker run --rm -it \
      -p 8080:8080 \
      -e GEMINI_API_KEY="your_gemini_api_key" \
      -e BIGQUERY_PROJECT_ID="your-gcp-project-id" \
      --name bq-tools-server \
      anyquery-server
    ```
    The server will be accessible at `http://localhost:8080`. You can use the `/health` endpoint to check if it's running.

## Core Dependencies

This project relies on several key Rust crates:

*   [**`tokio`**](https://crates.io/crates/tokio): For the asynchronous runtime.
*   [**`axum`**](https://crates.io/crates/axum): For the web server framework.
*   [**`reqwest`**](https://crates.io/crates/reqwest): For making HTTP requests to the Gemini API.
*   [**`gcp-bigquery-client`**](https://crates.io/crates/gcp-bigquery-client): For interacting with the Google BigQuery API.
*   [**`serde`**](https://crates.io/crates/serde): For serializing and deserializing data.
*   [**`tracing`**](https://crates.io/crates/tracing): For application-level logging.

## License

This project is licensed under the MIT License.
