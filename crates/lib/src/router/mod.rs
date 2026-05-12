//! # Domain Classifier Router
//!
//! Embedding-based domain classification for prompt routing.
//! Combines keyword overlap with vector embedding similarity
//! to classify prompts into domains for microgpt-rs routing.
//!
//! ## Usage
//!
//! ```ignore
//! use anyrag::router::{HybridClassifier, DomainDefinition, ScoredDomain};
//!
//! let classifier = HybridClassifier::new();
//! let scores = vec![ScoredDomain { ... }];
//! let result = classifier.classify_from_scores(scores)?;
//! ```

pub mod classifier;
pub mod hybrid;
pub mod types;

pub use classifier::DomainClassifier;
pub use hybrid::{HybridClassifier, ScoredDomain};
pub use types::{
    ClassificationResult, ClassifyError, DomainDefinition, DomainHints, DomainScore,
    InferenceBudget, ReasoningPolicy, TruncationMode, TruncationPolicy,
};
