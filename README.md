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

## Running Tests

You can run all tests for the entire workspace from the root directory:

```sh
cargo test --workspace
```

For instructions on how to run tests for a specific crate, please see its respective `README.md`.

## License

This project is licensed under the MIT License.