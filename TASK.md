# TASK.md: Actionable Development Plan

This document breaks down the architectural goals from `PLAN.md` into concrete, actionable tasks. Each task represents a distinct step in the refactoring and development process.

---

### Epic 1: Core Architectural Refactoring

**Goal**: Establish a clean separation between the `server` (web layer) and `lib` (core logic layer).

-   [ ] **Task 1.1: Consolidate Business Logic into `anyrag` (lib)**
    -   **Action**: Move all non-HTTP logic from `anyrag-server` into the `anyrag` crate.
    -   **Details**: This includes ingestion orchestration, search algorithms, RAG pipelines, and provider interactions. The `anyrag` crate should have no dependency on `axum`.
    -   **Acceptance Criteria**: The `anyrag` crate can be used by `anyrag-cli` without pulling in any web-related dependencies.

-   [ ] **Task 1.2: Simplify `anyrag-server` to a Thin Web Layer**
    -   **Action**: Refactor all HTTP handlers in `anyrag-server`.
    -   **Details**: Handlers should only be responsible for (1) Deserializing requests, (2) Authenticating users, (3) Calling the corresponding function in the `anyrag` library, and (4) Serializing the response.
    -   **Acceptance Criteria**: No core business logic remains in `anyrag-server/src/handlers`.

-   [ ] **Task 1.3: Decouple `lib` from `AppState`**
    -   **Action**: Refactor functions in `anyrag` to accept dependencies (like `SqliteProvider` or `AiProvider`) as arguments.
    -   **Details**: The library should not know about the server's `AppState`. The `AppState` will be responsible for creating and holding provider instances, which are then passed to the library functions.
    -   **Acceptance Criteria**: The `anyrag` library has no dependency on the `AppState` struct.

---

### Epic 2: Implement Plugin Architecture for Ingestion

**Goal**: Make the ingestion system modular and extensible by treating each data source as a plugin.

-   [x] **Task 2.1: Define a Generic `Ingestor` Trait**
    -   **Action**: Create a new trait `Ingestor` in `anyrag/src/ingest/mod.rs`.
    -   **Details**: The trait should define a common interface for all ingestion plugins, such as `async fn ingest(&self, source: &str) -> Result<Output, Error>`.
    -   **Acceptance Criteria**: A clear, generic trait for ingestion exists in the `anyrag` library.

-   [x] **Task 2.2: Isolate `github` Logic into a Plugin Crate**
    -   **Action**: Create a new crate at `crates/github` and name it `anyrag-github` in its `Cargo.toml`.
    -   **Details**: Move all logic related to cloning, parsing, and storing GitHub examples into this new crate, following the flat `crates/` directory structure.
    -   **Acceptance Criteria**: The `github` ingestion logic is fully self-contained in the `anyrag-github` crate.

-   [x] **Task 2.3: Implement the `Ingestor` Trait for `github`**
    -   **Action**: In the `crates/github` crate, create a `GithubIngestor` struct and implement the `Ingestor` trait for it.
    -   **Details**: The `ingest` method will encapsulate the existing `run_github_ingestion` pipeline.
    -   **Acceptance Criteria**: The `anyrag` library can use the `GithubIngestor` through the generic trait.

-   [x] **Task 2.4: Isolate `pdf` Logic into a Plugin Crate**
    -   **Action**: Create an `anyrag-pdf` crate and move the PDF ingestion pipeline from `anyrag-lib` and the handler logic into it.
    -   **Details**: The new crate should implement the `Ingestor` trait. The server handler will be updated to call it.
    -   **Acceptance Criteria**: PDF ingestion is a self-contained plugin.

-   [x] **Task 2.5: Isolate `web` Logic into a Plugin Crate**
    -   **Action**: Create an `anyrag-web` crate for URL ingestion.
    -   **Details**: Move the `run_ingestion_pipeline` logic for web content into this crate and implement the `Ingestor` trait.
    -   **Acceptance Criteria**: Web ingestion is a self-contained plugin.

-   [x] **Task 2.6: Isolate `rss` Logic into a Plugin Crate**
    -   **Action**: Create an `anyrag-rss` crate.
    -   **Details**: Move the `ingest_from_url` logic for RSS feeds into this crate and implement the `Ingestor` trait.
    -   **Acceptance Criteria**: RSS ingestion is a self-contained plugin.

-   [x] **Task 2.7: Isolate `sheet` Logic into a Plugin Crate**
    -   **Action**: Create an `anyrag-sheets` crate.
    -   **Details**: Move the Google Sheets ingestion logic into this crate and implement the `Ingestor` trait.
    -   **Acceptance Criteria**: Google Sheets ingestion is a self-contained plugin.

-   [x] **Task 2.8: Isolate `text` Logic into a Plugin Crate**
    -   **Action**: Create an `anyrag-text` crate.
    -   **Details**: Move the raw text chunking and ingestion logic into this crate and implement the `Ingestor` trait.
    -   **Acceptance Criteria**: Text ingestion is a self-contained plugin.

-   [x] **Task 2.9: Isolate `firebase` Logic into a Plugin Crate**
    -   **Action**: Create an `anyrag-firebase` crate.
    -   **Details**: Move the Firestore dump logic into this crate and implement the `Ingestor` trait.
    -   **Acceptance Criteria**: Firebase ingestion is a self-contained plugin.

---

### Epic 3: Refine Type Scoping and Management

**Goal**: Improve maintainability by clearly scoping types as either shared public models or crate-local internal types.

-   [ ] **Task 3.1: Centralize Shared Public Types**
    -   **Action**: Identify and move all types that are part of `anyrag`'s public API or are shared between multiple crates into `anyrag/src/types.rs`.
    -   **Details**: This includes structs like `SearchResult`, `ExecutePromptOptions`, and `User`.
    -   **Acceptance Criteria**: The `anyrag/src/types.rs` module serves as the single source of truth for the workspace's public data models.

-   [ ] **Task 3.2: Establish Crate-Local Types**
    -   **Action**: For each crate (e.g., `anyrag-github`, `anyrag-server`), create a local `src/types.rs` for internal-only data structures.
    -   **Details**: Types that are only used within a single crate should be moved here to improve encapsulation.
    -   **Acceptance Criteria**: Each crate manages its own internal types, reducing unnecessary exposure across the workspace.

-   [ ] **Task 3.3: Update All Imports**
    -   **Action**: Perform a workspace-wide search and replace to update all import paths to use the new canonical locations.
    -   **Acceptance Criteria**: The project compiles successfully with the refined import paths.

---

### Epic 4: Introduce Feature Flags

**Goal**: Allow for conditional compilation to create smaller, specialized binaries.

-   [ ] **Task 4.1: Configure Feature Flags in `Cargo.toml`**
    -   **Action**: In `anyrag/Cargo.toml` and `anyrag-server/Cargo.toml`, define feature flags for each ingestion plugin (e.g., `github`, `pdf`).
    -   **Details**: The `default` feature should enable all ingestion plugins. Each plugin crate will be an optional dependency tied to its feature flag.
    -   **Acceptance Criteria**: The `Cargo.toml` files contain a `[features]` section mapping features to optional dependencies.

-   [ ] **Task 4.2: Apply Conditional Compilation to Code**
    -   **Action**: Use `#[cfg(feature = "...")]` attributes to conditionally compile code related to each plugin.
    -   **Details**: This applies to API routes in `anyrag-server/src/router.rs` and module imports in `anyrag/src/lib.rs`.
    -   **Acceptance Criteria**: `cargo build --no-default-features --features github` compiles successfully without including `pdf` or `rss` code.

---

### Epic 5: Finalize and Document

**Goal**: Ensure the new architecture is well-documented and verified.

-   [ ] **Task 5.1: Update All `README.md` Files**
    -   **Action**: Revise the `README.md` in the root, `anyrag-server`, `anyrag-lib`, and each new plugin crate.
    -   **Details**: The documentation should clearly explain the new architecture, separation of concerns, and how to use the feature flags.
    -   **Acceptance Criteria**: All `README.md` files are up-to-date and accurately reflect the refactored structure.

-   [ ] **Task 5.2: Run Full Workspace Tests and Diagnostics**
    -   **Action**: Execute `cargo test --workspace` and `cargo clippy --workspace -- -D warnings`.
    -   **Details**: Fix any failing tests or clippy warnings that arose from the refactoring.
    -   **Acceptance Criteria**: All tests pass and the codebase is free of clippy warnings.

---

### Epic 6: Configuration and Constants Refactoring

**Goal**: Eliminate hardcoded "magic strings" and centralize configuration values according to `Rule 2.6`.

-   [x] **Task 6.1: Centralize Database Paths**
    -   **Action**: Replace hardcoded database directory strings like `"db/github_ingest"` with shared constants.
    -   **Details**: Define a `const GITHUB_DB_DIR: &str = "db/github_ingest";` in a suitable, shared location and update all usages in `anyrag-github`, `anyrag-cli`, and `anyrag-server`.
    -   **Acceptance Criteria**: The string literal `"db/github_ingest"` no longer appears in the codebase.