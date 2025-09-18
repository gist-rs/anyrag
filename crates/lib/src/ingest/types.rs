//! # Shared Ingestion Types
//!
//! This module contains shared data structures used by various ingestion plugins,
//! particularly for parsing responses from LLM-based processing steps like
//! metadata extraction.

use serde::Deserialize;

/// Represents a single piece of extracted metadata (e.g., an entity or keyphrase).
/// This is used to deserialize the JSON response from the metadata extraction LLM call.
#[derive(Deserialize, Debug)]
pub struct ContentMetadata {
    #[serde(rename = "type")]
    #[serde(default)]
    pub metadata_type: String,
    #[serde(default)]
    pub subtype: String,
    #[serde(default)]
    pub value: String,
}

/// Represents the top-level structure of the metadata extraction LLM response.
/// Some models wrap the metadata array in a top-level object.
#[derive(Deserialize, Debug)]
pub struct MetadataResponse {
    #[serde(default)]
    pub metadata: Vec<ContentMetadata>,
}
