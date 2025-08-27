//! # Shared Ingestion Modules
//!
//! This module contains common logic that is reused by multiple ingestion sources.

pub mod google_sheets;

pub use google_sheets::{construct_export_url_and_table_name, download_csv, SheetError};
