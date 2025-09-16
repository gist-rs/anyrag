//! # anyrag-github: GitHub Ingestion and Search Crate
//!
//! This crate contains all functionality related to ingesting code examples
//! from public GitHub repositories and searching them for Retrieval-Augmented
//! Generation (RAG).

pub mod cli;
pub mod ingest;

// Re-export the main functions for easy access from other crates.
pub use ingest::{run_github_ingestion, search_examples, types};
