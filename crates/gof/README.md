# `gof` Crate: Project-Aware RAG

The `gof` (Gist Of) crate provides a powerful command-line tool to automatically build a Retrieval-Augmented Generation (RAG) knowledge base from your Rust project's dependencies. It transforms the tedious process of searching through documentation and examples for multiple libraries into a single, seamless workflow.

## The Core Idea

When working on a Rust project, you often need to find code examples for the specific versions of the libraries you are using. This typically involves manually searching on GitHub, docs.rs, or other websites.

`gof` automates this entire process:

1.  It reads your project's `Cargo.toml`.
2.  It resolves each dependency to its source code repository (e.g., GitHub).
3.  It intelligently clones each repository at the exact version you are using.
4.  It extracts all relevant Rust code examples from documentation, tests, and example files.
5.  It stores this curated knowledge in a local, searchable database.

This creates a project-specific knowledge base that can be queried to find highly relevant code examples instantly.

---

## Commands

### `gof example`

This is the main ingestion command. It analyzes your project's dependencies and fetches all code examples. You should run this command once to build the initial knowledge base for your project.

**Usage:**

```sh
# Run from the root of your Rust project
cargo run -p gof -- example
```

This command will:
1.  Find the `Cargo.toml` in the current directory.
2.  Query `crates.io` to find the repository URL for each dependency.
3.  Ingest examples from all dependencies in parallel.

**Options:**

*   `--path <PATH>`: Specify a path to a different `Cargo.toml` file.
*   `--embedding-api-url <URL>`: (Optional) Provide an embedding API endpoint. If set, vector embeddings will be generated for all examples, enabling semantic search.
*   `--embedding-model <MODEL>`: (Required if embedding URL is set) The name of the embedding model to use.

**Example with Embeddings:**

```sh
# Ensure the embedding server is running and env vars are set
export EMBEDDINGS_API_URL="http://localhost:1234/v1/embeddings"
export EMBEDDINGS_MODEL="text-embedding-qwen3-embedding-8b"

cargo run -p gof -- example
```

### `gof mcp` (Model Context Protocol)

This command is **not designed for direct human use**. It implements a machine-readable **Model Context Protocol (MCP)**, intended to be used by other programs, such as code editors (e.g., Zed) or plugins.

It acts as a local RAG server, taking a query and returning a structured JSON object containing the most relevant code examples from the ingested knowledge base.

**Usage:**

The command requires a query and a list of repository names (as determined by the ingestion process) to search within.

```sh
# Set environment variables required for the search pipeline
export AI_API_URL="http://localhost:1234/v1/chat/completions"
export AI_MODEL="qwen3-coder-30b-a3b-instruct-mlx"
export EMBEDDINGS_API_URL="http://localhost:1234/v1/embeddings"
export EMBEDDINGS_MODEL="text-embedding-qwen3-embedding-8b"

# Search for examples related to the Turso client within its repository
cargo run -p gof -- mcp "how to connect to turso" --repos tursodatabase-turso
```

**Output:**

*   On success, it prints a JSON object to `stdout`:
    ```json
    {
      "results": [
        {
          "source_file": "examples/hello.rs",
          "handle": "example_file:examples/hello.rs",
          "content": "use libsql::Builder;\n\nasync fn main() {\n    let db = Builder::new_remote(\"my-turso-db.turso.io\", \"auth-token\").build().await.unwrap();\n    let conn = db.connect().unwrap();\n    conn.execute(\"CREATE TABLE IF NOT EXISTS users (id INT, name TEXT)\", ()).await.unwrap();\n    conn.execute(\"INSERT INTO users VALUES (1, 'Alice')\", ()).await.unwrap();\n}",
          "score": 0.912
        }
      ]
    }
    ```

*   On failure, it prints a JSON error object to `stderr` and exits with a non-zero status code:
    ```json
    {
      "error": {
        "code": "SearchError",
        "message": "The --repos flag must be provided with a list of repository names to search."
      }
    }
    ```

## Typical Workflow

1.  Navigate to your Rust project's root directory.
2.  Run `cargo run -p gof -- example` to ingest all examples from your dependencies. This may take some time on the first run.
3.  Integrate an editor plugin or other tool to call `gof mcp` with your queries to get instant, context-aware code examples without leaving your development environment.
