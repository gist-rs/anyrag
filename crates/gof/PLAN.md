# PLAN.md: `gof` Crate Development Plan

This document outlines the development plan for a new CLI tool, `gof` (Gist Of), designed to automate the creation of a Retrieval-Augmented Generation (RAG) knowledge base from a Rust project's dependencies.

---

## 1. Vision & Goal

The current `anyrag-cli dump github` command is powerful but manual, requiring the user to specify one repository and version at a time. The goal of `gof` is to create an intelligent, project-aware tool that significantly enhances developer productivity.

`gof` will inspect a project's `Cargo.toml`, automatically identify all its dependencies, and ingest code examples from each one at the correct version. This transforms the manual process into a single command, creating an instant, comprehensive, and searchable knowledge base tailored to the user's project.

This aligns with the core philosophy of `anyrag`: to create a robust, modular ecosystem for RAG.

## 2. Architecture & Crate Structure

In adherence with project standards (`RULES.md`, `IDEA.md`), the `gof` tool will be architected as follows:

-   **New Crate**: A new binary crate, `gof`, will be created in the `crates/` directory.
-   **Thin Binary (`main.rs`)**: `src/main.rs` will serve as a thin entry point, responsible only for parsing CLI arguments (`clap`), setting up logging, and dispatching to the core logic. This adheres to **Rule 2.2**.
-   **Core Logic (`lib.rs`)**: All application logic (parsing, API calls, orchestration) will reside in `src/lib.rs` and its submodules.
-   **Dependencies**: `gof` will leverage the existing `anyrag` ecosystem and other high-quality crates:
    -   `anyrag-github`: To use the existing, robust ingestion pipeline.
    -   `tokio`: For parallel, asynchronous ingestion of multiple repositories.
    -   `toml`: For parsing the project's `Cargo.toml` file.
    -   `crates_io_api`: To reliably resolve crate names to their source repository URLs.
    -   `clap`: For the command-line interface.

## 3. Core Commands and Functionality

### Command 1: `gof example` (The Ingestion Command)

This command will be the primary entry point for a user. It will execute a four-phase pipeline:

1.  **Phase 1: Dependency Parsing**:
    -   Locate and parse the `Cargo.toml` in the current working directory.
    -   Extract a list of all crate names and their version specifications from `[dependencies]` and `[dev-dependencies]`.

2.  **Phase 2: Repository URL Resolution**:
    -   For each dependency, use the `crates_io_api` crate to query the `crates.io` registry.
    -   Extract the official `repository` URL from the crate's metadata. This is the single source of truth and is more reliable than any other method.
    -   This step will produce a definitive list of `(repository_url, version)` pairs.

3.  **Phase 3: Parallel Ingestion**:
    -   Spawn a `tokio` task for each repository-version pair.
    -   Each task will invoke the `anyrag_github::run_github_ingestion` function. This heavily reuses existing, tested code, which handles:
        -   Git cloning and checking out the precise version.
        -   Extracting examples from READMEs, tests, doc comments, and example files.
        -   Resolving duplicate code examples based on source priority.
        -   Storing the results in a dedicated, versioned SQLite database.
        -   (Optional) Generating vector embeddings.

4.  **Phase 4: Context File Generation**:
    -   Upon successful ingestion of a repository, generate a consolidated `[repo-name]-[version]-context.md` file. This provides immediate, human-readable value and serves as the input for the search functionality.

### Command 2: `gof mcp` (Model Context Protocol)

This command is not a conventional CLI tool for human use. Instead, it implements the **Model Context Protocol (MCP)**, a machine-readable interface designed to be consumed by other applications, particularly code editors like Zed. Its purpose is to serve as a fast, local, project-aware RAG backend.

To achieve this, `gof mcp` will adhere to the following principles:

1.  **Structured JSON I/O**: The command will **only** communicate via JSON.
    -   **Success**: On a successful query, it will print a structured JSON object to `stdout` and exit with code 0.
    -   **Failure**: On any error (e.g., database not found, search logic fails), it will print a structured JSON error object to `stderr` and exit with a non-zero exit code.

2.  **Defined Schemas**:
    -   **Success Schema (`stdout`)**:
        ```json
        {
          "results": [
            {
              "repository": "https://github.com/tursodatabase/turso",
              "version": "v0.1.5",
              "source_file": "tests/auth_test.rs",
              "handle": "test:tests/auth_test.rs:test_login_flow",
              "content": "let db = Client::open(\"file:local.db\");",
              "score": 0.897
            }
          ]
        }
        ```
    -   **Error Schema (`stderr`)**:
        ```json
        {
          "error": {
            "code": "SearchError",
            "message": "Failed to connect to repository database for 'tursodatabase-turso'."
          }
        }
        ```

3.  **Underlying Logic**: The command will still use the powerful `anyrag_github::search_examples` function as its backend, performing the hybrid search, but it will wrap the final output in the MCP JSON format.

This protocol-centric design makes `gof` a powerful, extensible backend for building AI-native developer experiences directly within an editor.

## 4. Development Steps

Development will proceed in the following order:

1.  **Setup**: Create the `gof` crate with `Cargo.toml` and placeholder `main.rs`/`lib.rs`.
2.  **CLI Scaffolding**: Implement the `clap` argument structure for `gof example` and `gof mcp`.
3.  **Dependency Parser**: Implement the logic to read and parse `Cargo.toml`.
4.  **Repository Resolver**: Implement the logic to query `crates.io` and get repository URLs.
5.  **Orchestrator**: Implement the parallel ingestion logic in `gof example`.
6.  **MCP Interface**: Implement the `gof mcp` command, ensuring it adheres to the JSON protocol for both success and error cases.
7.  **Testing**: Add unit and integration tests for the new logic.