//! # Domain Classifier Trait
//!
//! Defines the `DomainClassifier` trait for embedding-based domain classification.
//! Implementations combine keyword overlap with vector embedding similarity
//! to classify prompts into domains.

use async_trait::async_trait;

use super::types::{ClassificationResult, ClassifyError, DomainDefinition};

/// Classifies a prompt into a domain using semantic search + keyword overlap.
///
/// Implementations should blend keyword-based scoring with embedding similarity
/// to produce a ranked list of domain scores. The trait is `Send + Sync` so
/// it can be stored in axum's `AppState`.
#[async_trait]
pub trait DomainClassifier: Send + Sync {
    /// Classify a prompt against candidate domains.
    ///
    /// Returns the best-matching domain with confidence score,
    /// matched slots, and alternative domain scores.
    async fn classify(
        &self,
        prompt: &str,
        candidate_domains: &[DomainDefinition],
    ) -> Result<ClassificationResult, ClassifyError>;
}
