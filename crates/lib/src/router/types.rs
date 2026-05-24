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

/// Per-domain inference budget parameters.
///
/// Controls how much compute a consumer (e.g., katgpt-rs) should spend
/// on inference for this domain. All fields optional — `None` means
/// "consumer decides" (use local defaults).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InferenceBudget {
    /// Maximum tokens for tree search exploration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree_budget: Option<usize>,
    /// Number of drafts to look ahead during speculative decoding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft_lookahead: Option<usize>,
    /// Threshold for screening/pruning draft candidates [0.0, 1.0].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screening_threshold: Option<f32>,
    /// Sampling temperature for generation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Single scalar [0.0, 1.0] that maps to explicit fields via `resolve()`.
    /// Higher = more compute. If set and explicit fields are None, derive from beta.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beta: Option<f32>,
}

impl InferenceBudget {
    /// Resolve the budget: if `beta` is set and `tree_budget` is None, derive from beta.
    /// Otherwise return a clone with explicit values as-is.
    pub fn resolve(&self) -> InferenceBudget {
        match self.beta {
            Some(beta) if self.tree_budget.is_none() => Self::from_beta(beta),
            _ => self.clone(),
        }
    }

    /// Create an `InferenceBudget` from a single scalar beta [0.0, 1.0].
    ///
    /// Monotonic mapping: higher beta = more compute.
    pub fn from_beta(beta: f32) -> InferenceBudget {
        let draft_lookahead = (beta * 15.0).round() as usize;
        InferenceBudget {
            tree_budget: Some((beta * 5000.0).round() as usize),
            draft_lookahead: if draft_lookahead > 0 {
                Some(draft_lookahead)
            } else {
                None
            },
            screening_threshold: if beta > 0.3 {
                Some(1.0 - beta * 0.5)
            } else {
                None
            },
            temperature: Some(beta),
            beta: Some(beta),
        }
    }
}

/// Truncation mode — whether the limit is measured in tokens or bytes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TruncationMode {
    Tokens,
    Bytes,
}

/// Truncation policy for context window management.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TruncationPolicy {
    /// Truncation mode: "tokens" preserves more context than "bytes".
    pub mode: TruncationMode,
    /// Maximum limit before truncation applies.
    pub limit: u32,
}

/// Policy for retaining reasoning/thinking content across turns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReasoningPolicy {
    /// Preserve reasoning content behind tool calls.
    #[serde(default)]
    pub keep_on_tool_calls: bool,
    /// Preserve reasoning content on ordinary turns.
    #[serde(default)]
    pub keep_on_plain: bool,
}

/// Agent hints for domain behavior optimization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainHints {
    /// Latency sensitivity [0.0, 1.0]. Higher = more interactive domain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_sensitivity: Option<f32>,
    /// Enable speculative prompt prefill/compression.
    #[serde(default)]
    pub speculative_prefill: bool,
}

/// Score for a single domain candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainScore {
    /// Domain name.
    pub domain: String,
    /// Confidence score in [0.0, 1.0].
    pub confidence: f32,
    /// Per-request inference budget for this domain (if configured).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inference: Option<InferenceBudget>,
}

/// Result of classifying a prompt into a domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Inference budget for the winning domain (if configured).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inference: Option<InferenceBudget>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inference_budget_serde_roundtrip() {
        let budget = InferenceBudget {
            tree_budget: Some(5000),
            draft_lookahead: Some(12),
            screening_threshold: Some(0.3),
            temperature: Some(0.8),
            beta: Some(0.8),
        };
        let json = serde_json::to_string(&budget).unwrap();
        let deserialized: InferenceBudget = serde_json::from_str(&json).unwrap();
        assert_eq!(budget, deserialized);
    }

    #[test]
    fn test_inference_budget_none_defaults() {
        // Empty JSON object should give all None fields
        let json = "{}";
        let budget: InferenceBudget = serde_json::from_str(json).unwrap();
        assert_eq!(budget.tree_budget, None);
        assert_eq!(budget.draft_lookahead, None);
        assert_eq!(budget.screening_threshold, None);
        assert_eq!(budget.temperature, None);
        assert_eq!(budget.beta, None);
    }

    #[test]
    fn test_inference_budget_from_beta() {
        let budget = InferenceBudget::from_beta(0.8);
        assert_eq!(budget.tree_budget, Some(4000)); // 0.8 * 5000 = 4000
        assert_eq!(budget.draft_lookahead, Some(12)); // 0.8 * 15 = 12
        assert_eq!(budget.screening_threshold, Some(0.6)); // 1.0 - 0.8*0.5 = 0.6
        assert_eq!(budget.temperature, Some(0.8));
        assert_eq!(budget.beta, Some(0.8));

        // Low beta: no screening_threshold, no draft_lookahead
        let low = InferenceBudget::from_beta(0.1);
        assert_eq!(low.tree_budget, Some(500)); // 0.1 * 5000 = 500
        assert_eq!(low.draft_lookahead, Some(2)); // 0.1 * 15 = 1.5 → 2
        assert_eq!(low.screening_threshold, None); // beta <= 0.3
        assert_eq!(low.temperature, Some(0.1));
        assert_eq!(low.beta, Some(0.1));
    }

    #[test]
    fn test_inference_budget_resolve_with_explicit_values() {
        // When explicit tree_budget is set, resolve should keep it (ignore beta)
        let budget = InferenceBudget {
            tree_budget: Some(9999),
            draft_lookahead: None,
            screening_threshold: None,
            temperature: None,
            beta: Some(0.8),
        };
        let resolved = budget.resolve();
        assert_eq!(resolved.tree_budget, Some(9999));
        // Should clone as-is, not derive from beta
        assert_eq!(resolved.draft_lookahead, None);
        assert_eq!(resolved.beta, Some(0.8));
    }

    #[test]
    fn test_inference_budget_resolve_from_beta() {
        // When beta is set and tree_budget is None, resolve derives from beta
        let budget = InferenceBudget {
            tree_budget: None,
            draft_lookahead: None,
            screening_threshold: None,
            temperature: None,
            beta: Some(0.6),
        };
        let resolved = budget.resolve();
        assert_eq!(resolved.tree_budget, Some(3000)); // 0.6 * 5000 = 3000
        assert_eq!(resolved.draft_lookahead, Some(9)); // 0.6 * 15 = 9
        assert_eq!(resolved.screening_threshold, Some(0.7)); // 1.0 - 0.6*0.5 = 0.7
        assert_eq!(resolved.temperature, Some(0.6));
        assert_eq!(resolved.beta, Some(0.6));
    }

    #[test]
    fn test_truncation_policy_serde_roundtrip() {
        let policy = TruncationPolicy {
            mode: TruncationMode::Tokens,
            limit: 10000,
        };
        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: TruncationPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(policy, deserialized);

        // Also verify snake_case serialization
        assert!(json.contains("\"tokens\""));
    }

    #[test]
    fn test_reasoning_policy_serde_roundtrip() {
        let policy = ReasoningPolicy {
            keep_on_tool_calls: true,
            keep_on_plain: false,
        };
        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: ReasoningPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(policy, deserialized);
    }

    #[test]
    fn test_domain_hints_serde_roundtrip() {
        let hints = DomainHints {
            latency_sensitivity: Some(0.8),
            speculative_prefill: true,
        };
        let json = serde_json::to_string(&hints).unwrap();
        let deserialized: DomainHints = serde_json::from_str(&json).unwrap();
        assert_eq!(hints, deserialized);
    }
}
