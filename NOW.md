# NOW: Implementing Server API for GitHub Ingestion

This document tracks the implementation of the features outlined in `PLAN.md`.

## Phase 1: Core Library Implementation (Completed)

- [x] **Setup Module and Core Types**
    - [x] Create `anyrag/crates/lib/src/github_ingest` module.
    - [x] Define core data structures in `types.rs`.
    - [x] Implement database logic in `storage.rs`.
    - [x] Implement repository cloning logic in `crawler.rs`.
    - [x] Implement file discovery logic in `extractor.rs`.
- [x] **Implement Example Extraction Logic**
    - [x] Implement `extractor.rs` to parse `README.md` files for Rust code blocks.
    - [x] Implement `extractor.rs` to parse `/examples/*.rs` files.
    - [x] Implement `extractor.rs` to parse Rust doc comments (`///`, `//!`).
    - [x] Implement `extractor.rs` to parse Rust test files (`/tests/*.rs`, `*_test.rs`).
    - [x] Implement conflict resolution logic based on `ExampleSourceType` priority.
- [x] **Create Main Ingestion Orchestrator**
    - [x] Create a main function in `github_ingest/mod.rs` that takes an `IngestionTask`.
    - [x] This function will call the `Crawler`, `Extractor`, and `StorageManager` in sequence.
    - [x] Add logic to determine the latest version if none is specified in the task.
    - [x] Implement `Cargo.toml` parsing as a version fallback.

## Phase 2: Server API and RAG Integration (In Progress)

- [x] **Implement API Endpoints**
    - [x] `POST /ingest/github`: Create a handler that accepts a URL and version, and kicks off the ingestion task.
    - [x] `GET /examples/{repo_name}/{version}`: Create a handler to generate and return the consolidated Markdown file.
    - [x] `POST /search/examples`: Create the RAG handler for querying examples. (Placeholder implemented)

- [x] **Integrate Multi-DB RAG Logic**
    - [x] Update the RAG pipeline to dynamically connect to the correct repository-specific database based on the request.
    - [x] Implement the two-stage RAG for multi-repository queries.

## Phase 3: Testing and Refinement

- [x] **Write Integration Tests**
    - [x] Write a test for the full ingestion pipeline against a mock or real public repository.
    - [x] Fix flaky tests in `extractor_test.rs`.
    - [x] Write E2E tests for the new API endpoints (`/ingest/github`, `/examples/{repo_name}/{version}`).
    - [x] Write E2E tests for the new API endpoints (`/search/examples`). (Placeholder test implemented)
    - [x] Write a test for a versioned RAG query.
    - [x] Write a test for a multi-repo RAG query.

## Phase 4: CLI Refinement and GitHub Integration Command (Completed)

- [x] **Refine CLI Structure**
  - [x] Separate Firebase-related logic into a new `cli/src/firebase.rs` module.
  - [x] Update `cli/src/main.rs` to use the new module.
- [x] **Implement `dump github` Command**
  - [x] Add `github` subcommand to the `dump` command in `cli/src/main.rs`.
  - [x] Implement the logic to call the `anyrag::github_ingest::run_github_ingestion` function.
  - [x] Create a new handler function for the `dump github` command.
- [x] **Generate Consolidated Markdown Output**
  - [x] Add functionality to the CLI to generate the `tursodatabase-turso-context.md` file from the ingested examples.
- [x] **Update Documentation**
  - [x] Rewrite `crates/cli/README.md` to include instructions and examples for the new `dump github` command.
- [x] **Testing**
  - [x] Add a unit test for the CLI argument parsing.
  - [x] Write an integration test for the end-to-end `dump github` workflow.