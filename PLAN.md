# Project: GitHub Example Ingestion and RAG System

## 1. Goal

The primary goal is to create a system that can crawl public GitHub repositories, intelligently extract versioned "how-to-use examples" from various sources (documentation, code, tests), store them in a queryable format, and serve them through an API. This will provide LLMs with accurate, up-to-date context for code generation tasks, overcoming the issue of outdated knowledge.

## 2. Core Capabilities

### 2.1. GitHub Repository Crawler
- **Task:** Ingest content from a given public GitHub repository URL (e.g., `https://github.com/tursodatabase/turso`).
- **Details:** The crawler will be responsible for fetching the repository content. Initially, this can be done by cloning the repository into a temporary directory to access files.
- **Versioning:** It must be able to check out specific versions (Git tags/releases) or default to the latest version if none is specified. The versioning strategy will be:
    1.  Prioritize Git tags that follow semantic versioning (e.g., `v1.2.3`).
    2.  If no tags, fall back to the version specified in project files (e.g., `Cargo.toml`).
    3.  As a last resort, use the latest commit hash.

### 2.2. Example Extractor
- **Task:** Identify and extract code examples from multiple sources within a repository.
- **Source Prioritization:** The system will extract from the following sources and resolve conflicts by trusting code over documentation:
    1.  **Tests (`/tests` or inline):** Highest priority. Test code is executable and demonstrates real usage.
    2.  **Source Code Comments:** Examples found in doc comments (e.g., `//!` or `///` in Rust) are high priority.
    3.  **Dedicated Example Files (`/examples`):** Medium priority.
    4.  **README Files (`README.md`):** Lowest priority, as they can become outdated.
- **Conflict Handling:** When the same example (e.g., connecting to a database) is found in multiple places with different implementations, the version from the source with the highest priority will be used. Conflicts will be logged for review.

### 2.3. Versioned Storage
- **Task:** Store the extracted examples in a structured, versioned, and queryable format.
- **Database Strategy:** A multi-database approach will be used.
    -   **Main Metadata DB (`github_meta.db`):** A central SQLite database to track all ingested repositories.
        -   `repositories` table: `id`, `repo_name` (e.g., `tursodatabase-turso`), `url`, `db_path` (e.g., `db/tursodatabase-turso.db`).
    -   **Repository-Specific DBs:** A separate SQLite database for each repository, named after the repository (e.g., `db/tursodatabase-turso.db`). This isolates data and simplifies management.
        -   `generated_examples` table: `id`, `example_handle` (a unique name), `content`, `source_file`, `source_type` (test, readme, etc.), `version`.
        -   `example_embeddings` table: For storing vector embeddings to power RAG.

### 2.4. RAG Query Engine (Completed)
- **Task:** Provide an API to answer natural language questions about how to use the code.
- **Functionality:**
    1.  **Single-Repo Query:** Answer prompts like `"gimme a code helloworld example for turso"`. The system will perform a hybrid search on the latest version of the `tursodatabase-turso.db`.
    2.  **Versioned Query:** Answer prompts like `"show me turso connection examples for v0.8.0"`.
    3.  **Multi-Repo Query (Advanced):** Handle complex prompts that combine knowledge from multiple repositories, such as `"show me a hello world example using turso v1 and dioxus v2"`. This requires a two-stage RAG pipeline: first, identify the relevant repositories and versions, then perform a coordinated search across their respective databases.

### 2.5. Consolidated Markdown Export
- **Task:** Provide an API to generate a single, well-formatted Markdown file containing all `generated_examples` for a specific repository and version.
- **Naming Convention:** The output will be named `generated_examples_{repo_name}_{version}.md`.
- **Purpose:** This file can be used as a large context block for LLMs that have a large context window, providing a comprehensive overview of the library's usage.

## 3. Proposed Architecture & Implementation

- **Code Location:** A new module will be created at `anyrag/crates/lib/src/github_ingest/` to house this functionality.
- **Example Naming (`example_handle`):** To create a referable name for each example, we will use a convention like `{source_type}:{file_path}:{function_or_line_range}`. For example: `test:tests/auth.rs:test_login_flow`.
- **API Endpoints:**
    -   `POST /ingest/github`: Triggers the ingestion of a repository. Body: `{ "url": "...", "version": "..." }`.
    -   `GET /examples/{repo_name}`: Gets the consolidated Markdown for the latest version.
    -   `GET /examples/{repo_name}/{version}`: Gets the consolidated Markdown for a specific version.
    -   `POST /search/examples`: The RAG endpoint. Body: `{ "prompt": "...", "repos": ["tursodatabase-turso:v1.0.0", "dioxus-labs-dioxus:v0.4.0"] }`.

## 4. Unaddressed Considerations (Future Scope)

- **Scalability & Rate Limiting:** Heavy use of the GitHub API will require authentication (Personal Access Tokens) to avoid rate limits. The initial implementation can use unauthenticated cloning.
- **Language Support:** The initial implementation will focus on Rust projects (`.rs` files, `Cargo.toml`), but the architecture should be modular to allow for future support of other languages (e.g., TypeScript, Python).
- **Security:** Ingesting and executing code from arbitrary repositories carries risks. The system will only read and store content as text and will not execute any code.