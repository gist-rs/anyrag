//! # Shared Constants
//!
//! This module provides a centralized location for constants that are shared across
//! multiple crates in the `anyrag` workspace. Using these constants helps to avoid
//! "magic strings" and ensures consistency.

/// The root directory for all local databases.
pub const DB_DIR: &str = "db";

/// The directory for storing databases related to GitHub example ingestion.
pub const GITHUB_DB_DIR: &str = "db/github_ingest";

/// The directory for storing databases of processed chunks from GitHub examples.
pub const GITHUB_CHUNKS_DB_DIR: &str = "db/github_chunks";

/// The default path for the main application SQLite database.
pub const DEFAULT_DB_FILE: &str = "db/anyrag.db";
