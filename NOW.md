# Current Work In Progress: Search Refactoring

This document tracks the immediate next steps for completing the refactoring of the hybrid search functionality.

## Objective

The goal is to evolve the `hybrid_search` function and its surrounding logic to be more robust, performant, and user-configurable, as outlined in `PLAN.md`.

## Pending Tasks

-   [ ] **Refactor `hybrid_search` to use an `Options` struct:**
    -   Define a `HybridSearchOptions` struct to encapsulate the numerous parameters.
    -   Update the `hybrid_search` function signature to accept this struct.
    -   This will resolve the "too many arguments" warning and improve code clarity.

-   [ ] **Implement Parallel Execution and Soft Failure:**
    -   Within `hybrid_search`, use `tokio::join!` to run the keyword and vector searches concurrently.
    -   Ensure shared resources (like the database provider) are correctly handled across threads using `Arc`.
    -   Implement "soft failure": if one of the search tasks fails or panics, log the error but allow the function to proceed with the results from the successful task(s).

-   [ ] **Update All `hybrid_search` Call Sites:**
    -   Modify the calls in `crates/server/src/handlers/generation_handlers.rs` to use the new `HybridSearchOptions` struct.
    -   Modify the calls in `crates/server/src/handlers/knowledge.rs` to match the new signature.
    -   Update the example file `crates/lib/examples/knowledge.rs`.
    -   Update the integration test `crates/lib/tests/knowledge_search_logic_test.rs`.

-   [ ] **Final Diagnostic Run:**
    -   After all changes are implemented, run `cargo clippy` and `cargo test` to ensure the entire project is free of errors and warnings.