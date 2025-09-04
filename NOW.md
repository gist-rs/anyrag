#### **NOW: Migrate to `clap` and Implement Core Commands**

1.  **Refactor to `clap`**:
    *   **Action**: Replace `ratatui` and `crossterm` with `clap` in `crates/cli/Cargo.toml`.
    *   **Action**: Rework `crates/cli/src/main.rs` to parse arguments using `clap::Parser`.
    *   **Action**: Define the top-level `Cli` struct with subcommands: `login`, `dump`, and `process`.

2.  **Implement `login` Command**:
    *   **Action**: Create the `login` command handler.
    *   **Action**: Integrate the existing `auth::login()` function.
    *   **Action**: Ensure the JWT is stored in the OS keychain using the `keyring` crate upon success.

3.  **Research & Integrate Firebase Client**:
    *   **Action**: Investigate and select a suitable Rust crate for interacting with Google Firestore (e.g., `gcp_sdk`, `tonic`-based clients).
    *   **Action**: Add the chosen dependency to `crates/cli/Cargo.toml`.

4.  **Implement `dump firebase` Command**:
    *   **Action**: Create the command handler for `dump firebase`.
    *   **Action**: Implement the logic to authenticate with GCP/Firebase, connect to the specified project, and fetch data from a Firestore collection.
    *   **Action**: Implement the logic to write the fetched data into a new table in the local `anyrag.db` SQLite database.