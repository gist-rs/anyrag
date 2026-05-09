//! # Domain Classifier Types
//!
//! Core types for embedding-based domain classification.
//! Used by the `DomainClassifier` trait and the REST API handler.

use serde::{Deserialize, Serialize};

/// A domain definition for classification.
///
/// Each domain has a name, a set of keywords for keyword-based scoring,
/// and an optional list of slot names for embedding-based scoring against
/// slot documents.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DomainDefinition {
    /// Unique domain name (e.g., "rust_code", "py2rs", "sudoku").
    pub name: String,
    /// Keywords associated with this domain for keyword overlap scoring.
    pub keywords: Vec<String>,
    /// Slot names to search for embedding similarity scoring.
    /// If empty, embedding scoring is skipped for this domain.
    #[serde(default)]
    pub slots: Vec<String>,
}

/// Score for a single domain candidate.
#[derive(Debug, Clone, Serialize)]
pub struct DomainScore {
    /// Domain name.
    pub domain: String,
    /// Confidence score in [0.0, 1.0].
    pub confidence: f32,
}

/// Result of classifying a prompt into a domain.
#[derive(Debug, Clone, Serialize)]
pub struct ClassificationResult {
    /// The best-matching domain.
    pub domain: String,
    /// Confidence score for the top domain in [0.0, 1.0].
    pub confidence: f32,
    /// Slot names that matched via embedding similarity.
    #[serde(default)]
    pub matched_slots: Vec<String>,
    /// Alternative domain scores, sorted by confidence descending.
    /// Does not include the top domain.
    #[serde(default)]
    pub alternatives: Vec<DomainScore>,
}

/// Error type for domain classification.
#[derive(Debug, thiserror::Error)]
pub enum ClassifyError {
    /// No candidate domains were provided.
    #[error("No candidate domains provided")]
    NoCandidates,
    /// AI provider is unavailable for embedding generation.
    #[error("AI provider unavailable: {0}")]
    ProviderUnavailable(String),
    /// Embedding generation failed.
    #[error("Embedding generation failed: {0}")]
    EmbeddingFailed(String),
    /// Vector search failed.
    #[error("Vector search failed: {0}")]
    SearchFailed(String),
}
