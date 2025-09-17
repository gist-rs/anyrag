# RULES.md: Engineering and Development Guidelines

This document establishes the best practices, architectural principles, and development methodologies for the `anyrag` project. Adherence to these rules is crucial for maintaining a high-quality, scalable, and maintainable codebase. These guidelines apply to both human developers and AI-assisted coding partners.

---

## 1. Architectural Principles

### 1.1. Strict Separation of Concerns

The project is divided into distinct layers. Each layer has a single, well-defined responsibility.

-   **`anyrag-server` (The Web Layer)**: Its **only** job is to handle HTTP communication. It receives requests, validates them, authenticates the user, calls the appropriate function in the library, and serializes the response. It must **never** contain core business logic.
-   **`anyrag` (The Core Library)**: This is the brain of the application. It contains all business logic, orchestrates workflows (ingestion, search), and provides a stable, public API for the server and CLI to consume. It is completely agnostic of the web.
-   **Plugins (Feature Crates)**: Specialized functionalities, especially data ingestion (`github`, `pdf`, etc.), must be encapsulated in their own crates. This makes the system modular and easy to extend.

### 1.2. Plugin-First for Extensibility

New functionalities, particularly for data sources, should be implemented as self-contained plugins.

-   **Prefer Traits over Enums for Dispatch**: For behaviors that can be extended (like ingestion sources), define a generic `trait` in the core library (e.g., `trait Ingestor`). Each plugin then provides a struct that implements this trait. This is more extensible than adding a new variant to a giant `enum` in the core library.
-   **Self-Contained Logic**: A plugin crate should contain everything it needs to operate: its specific logic, its dependencies, and any prompts or configuration templates it requires.

### 1.3. Centralized Core Types

To prevent circular dependencies and create a single source of truth for our data models, all shared types must be centralized.

-   **Location**: `anyrag/src/types.rs`.
-   **Content**: This module should contain any `struct` or `enum` that is shared between the server, the library, and/or plugins (e.g., `SearchResult`, `Document`, `User`).

---

## 2. Development Process

### 2.1. Plan Before You Code

For any non-trivial feature or refactor, the following "top-down" process must be followed. This ensures clarity of thought before implementation begins.

1.  **`PLAN.md` (The "Why" and "What")**: Update or create a master plan that describes the high-level architectural vision and goals. What problem are we solving? What will the end state look like?
2.  **`TASK.md` (The "How")**: Break down the `PLAN.md` into a concrete, sequential list of actionable tasks. Each task should be small, specific, and verifiable.
3.  **Implementation**: Execute the tasks from `TASK.md` one by one.

### 2.2. Test Rigorously

-   **Unit Tests**: Each module, especially those with complex business logic, should have unit tests that verify its behavior in isolation.
-   **Integration Tests**: Test the interaction *between* components, particularly between the library and its plugins, and the library and its database provider.
-   **End-to-End (E2E) Tests**: Use API-level tests (e.g., via `curl` or a test client) to verify that the entire system works as expected from the user's perspective. For `anyrag-server`, this is the most critical form of testing.

### 2.3. Use Feature Flags for Modularity

All optional components, especially ingestion plugins, must be gated by Cargo feature flags.

-   **Granularity**: One feature per plugin (e.g., `ingest_github`).
-   **Default**: The `default` features in `Cargo.toml` should include all stable plugins for a complete out-of-the-box experience.
-   **Benefit**: This allows for compiling smaller, specialized binaries by disabling unneeded features (e.g., `cargo build --no-default-features --features ingest_pdf`).

---

## 3. Code and Configuration Best Practices

### 3.1. Contextual Data Co-location

To make the system easier to reason about, configuration and data should be located near the code that uses them.

-   **Prompts**: LLM prompt templates should reside within the crate and module that uses them. For example, prompts for PDF refinement belong in the `anyrag-pdf` crate, while prompts for GitHub example search belong in the `anyrag-github` crate. Only globally shared prompts (like RAG synthesis) should live in the core library.
-   **YAML as the Source of Truth**: For knowledge base ingestion, the primary goal is to convert unstructured source data into a structured, hierarchical YAML format. This YAML, stored as a single document in the database, becomes the rich, contextual "chunk" for RAG, far superior to arbitrary text splits.

### 3.2. Error Handling

-   Use the `thiserror` crate to create specific, descriptive error types for each module (e.g., `GitHubIngestError`, `KnowledgeError`).
-   Avoid using `anyhow::Error` or `Box<dyn Error>` in library function signatures. Return specific error types so the calling code can handle different failure modes gracefully. `anyhow::Error` is acceptable within `main.rs` or at the highest level of an application where the error is simply logged and the program exits.

### 3.3. Configuration Layers

The configuration system is designed to be layered for maximum flexibility:

1.  **Hardcoded Defaults**: The safest, most basic defaults (especially for prompts) are hardcoded as `const` strings in the library.
2.  **`config.<provider>.yml`**: These files provide sane, working defaults for specific AI providers (e.g., `config.gemini.yml`, `config.local.yml`). The user copies one to `config.yml`.
3.  **`config.yml`**: The primary user-facing configuration file. It is git-ignored and used to define providers and tasks.
4.  **`.env`**: For secrets and environment-specific variables (API keys, ports). These are substituted into `config.yml`.
5.  **`prompt.yml`**: An optional, git-ignored file that allows a user to override *only* the prompt strings for specific tasks without touching the rest of the configuration.