# AnyRag: Natural Language to SQL

This project is a Rust-based workspace designed to translate natural language prompts into BigQuery SQL queries and execute them. It uses Google's Gemini API for translation and the Google BigQuery API for execution.

## Workspace Crates

The workspace is divided into two main crates. For detailed information on configuration, setup, and usage, please refer to the `README.md` file within each crate's directory.

-   **[`anyrag`](crates/lib/README.md)**: The core library responsible for prompt-to-SQL conversion and BigQuery interaction.
-   **[`anyrag-server`](crates/server/README.md)**: A lightweight `axum` web server that exposes the library's functionality via a REST API, ready for containerized deployment.

## Project Structure

```
anyrag/
├── Cargo.toml         # Workspace configuration
├── crates/
│   ├── lib/           # The core logic library
│   │   ├── Cargo.toml
│   │   ├── README.md  <-- Library details
│   │   └── src/
│   └── server/        # The axum web server
│       ├── Cargo.toml
│       ├── README.md  <-- Server details
│       ├── Dockerfile
│       └── src/
└── README.md          # This file
```

## Deployment to Google Cloud Run

This project includes a comprehensive script to automate deployment to Google Cloud Run. The script handles creating secrets, setting IAM permissions, building the container, and deploying the service.

### Prerequisites

-   The [Google Cloud SDK](https://cloud.google.com/sdk/docs/install) must be installed and initialized.
-   You must have a Google Cloud project with billing enabled.
-   Your `crates/server/.env` file must be created and contain your `AI_API_KEY` and `BIGQUERY_PROJECT_ID`.

### How to Deploy

1.  **Make the script executable:**
    ```sh
    chmod +x deploy.sh
    ```

2.  **Run the deployment script, passing your Google Cloud Project ID as an argument:**
    ```sh
    ./deploy.sh your-gcp-project-id
    ```
    The script will guide you through the authentication process and handle all the necessary steps to get your service live. Upon completion, it will output the URL for your deployed service.

## Running Tests

You can run all tests for the entire workspace from the root directory:

```sh
cargo test --workspace
```

For instructions on how to run tests for a specific crate, please see its respective `README.md`.

## License

This project is licensed under the MIT License.