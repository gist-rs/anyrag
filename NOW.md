# Plan: Isolate GitHub Functionality into a Dedicated Crate

This plan outlines the steps to move all GitHub-related ingestion and search logic from the `anyrag` (lib) and `cli` crates into a new, dedicated crate named `anyrag-github`. This will improve modularity, simplify dependencies, and prepare for future Git-related features.

## 1. Create the New Crate

- **Action:** Create a new library crate within the `crates/` directory.
- **Name:** `github`
- **Path:** `crates/github`

## 2. Move Core Logic from `anyrag` (lib) to `github`

- **Action:** Move the entire `github_ingest` module directory.
- **From:** `crates/lib/src/github_ingest`
- **To:** `crates/github/src/`
- **Refactor:** Rename the `github_ingest` directory to `ingest` inside the new crate for clarity (`crates/github/src/ingest`). Expose the public functions (`run_github_ingestion`, `search_examples`) through `crates/github/src/lib.rs`.

## 3. Move CLI Logic from `cli` to `github`

- **Action:** Move the file containing the CLI command handler.
- **From:** `crates/cli/src/github.rs`
- **To:** `crates/github/src/cli.rs`
- **Refactor:**
    - The `GithubArgs` struct and `handle_dump_github` function will reside in `crates/github/src/cli.rs`.
    - `handle_dump_github` will become a public function that the `cli` crate can call.

## 4. Update Dependencies (`Cargo.toml`)

- **`anyrag/Cargo.toml` (Workspace Root):**
    - Add `crates/github` to the `workspace.members` array.

- **`anyrag/crates/lib/Cargo.toml`:**
    - **Remove** the following dependencies, as they are only used for GitHub ingestion: `git2`, `htmd`, `tempfile`, `semver`, `kuchikiki`, `html5ever`, `toml`, `scraper`.

- **`anyrag/crates/github/Cargo.toml`:**
    - **Add** the dependencies removed from `crates/lib/Cargo.toml`.
    - **Add** `anyrag` (the lib) as a dependency to access shared types like `SearchResult`.
    - **Add** `clap` for the `GithubArgs` struct.
    - **Add** other necessary dependencies like `tokio`, `tracing`, `anyhow`, `serde`, etc.

- **`anyrag/crates/cli/Cargo.toml`:**
    - **Add** a new dependency on the local `github` crate: `github = { path = "../github" }`.
    - **Remove** `anyrag` from dev-dependencies if it is no longer needed after moving github related code.

- **`anyrag/crates/server/Cargo.toml`:**
    - **Add** a new dependency on the local `github` crate: `github = { path = "../github" }`.

## 5. Refactor Code to Use the New Crate

- **`anyrag/crates/cli/src/main.rs`:**
    - Update the `DumpCommands::Github` enum to use `github::cli::GithubArgs`.
    - Update the `handle_dump` function to call `github::cli::handle_dump_github`.
    - Update `use` statements to import from the new `github` crate instead of the local module.

- **`anyrag/crates/server/src/handlers/ingest.rs`:**
    - Update `use` statements to import `run_github_ingestion`, `search_examples`, etc., from the `github` crate.
    - Update calls to these functions accordingly.

- **`anyrag/crates/lib/src/lib.rs`:**
    - Remove the `pub mod github_ingest;` line.

## 6. Move and Update Tests

- **Action:** Move the GitHub-related integration tests.
- **From:**
    - `crates/lib/tests/github_ingest/extractor_test.rs`
    - `crates/lib/tests/github_ingest_test.rs`
    - `crates/lib/tests/github_search_logic_test.rs`
- **To:** A new `tests/` directory inside `crates/github`.
- **Refactor:**
    - Update the `[[test]]` sections in `crates/lib/Cargo.toml` by removing the GitHub tests.
    - Add new `[[test]]` sections in `crates/github/Cargo.toml` for the moved tests.
    - Update `use` paths within the test files to reflect their new location.

## 7. Final Verification

- **Action:** Run all tests in the workspace to ensure no regressions.
- **Command:** `cargo test --workspace`
- **Action:** Run the `cli` and `server` to manually verify the `dump github` and `/search/examples` functionalities still work as expected.