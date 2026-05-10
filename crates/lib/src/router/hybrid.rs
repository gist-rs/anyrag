//! # Hybrid Domain Classifier
//!
//! Combines keyword overlap (30%) with embedding similarity (70%)
//! for domain classification. Provides pure scoring functions —
//! the handler is responsible for I/O (embedding generation, vector search).
//!
//! ## Scoring Pipeline
//!
//! 1. **Keyword score**: ratio of domain keywords found in the prompt (case-insensitive).
//! 2. **Embedding score**: top vector similarity score from searching the domain's slots.
//! 3. **Hybrid score**: `keyword * 0.3 + embedding * 0.7`.
//!    Falls back to keyword-only when embedding data is unavailable.

use super::types::{ClassificationResult, ClassifyError, DomainDefinition, DomainScore};

/// A domain with pre-computed scores, ready for ranking.
#[derive(Debug, Clone)]
pub struct ScoredDomain {
    /// Domain name.
    pub domain: String,
    /// Keyword overlap score in [0.0, 1.0].
    pub keyword_score: f32,
    /// Embedding similarity score in [0.0, 1.0], or `None` if unavailable.
    pub embedding_score: Option<f32>,
    /// Slot names that contributed to the embedding score.
    pub matched_slots: Vec<String>,
}

/// Hybrid classifier combining keyword overlap with embedding similarity.
///
/// Default weights: 30% keyword, 70% embedding.
/// Falls back to keyword-only (100%) when embedding data is `None`.
pub struct HybridClassifier {
    keyword_weight: f32,
    embedding_weight: f32,
}

impl Default for HybridClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl HybridClassifier {
    /// Create a new classifier with default weights (0.3 keyword, 0.7 embedding).
    pub fn new() -> Self {
        Self {
            keyword_weight: 0.3,
            embedding_weight: 0.7,
        }
    }

    /// Create a classifier with custom weights.
    ///
    /// Weights should sum to ~1.0 for normalized output.
    pub fn with_weights(keyword_weight: f32, embedding_weight: f32) -> Self {
        Self {
            keyword_weight,
            embedding_weight,
        }
    }

    /// Compute keyword overlap score for a prompt against a domain's keywords.
    ///
    /// Returns the ratio of matched keywords in [0.0, 1.0].
    /// Matching is case-insensitive substring matching.
    /// Returns 0.0 if the domain has no keywords.
    pub fn keyword_score(prompt: &str, domain: &DomainDefinition) -> f32 {
        if domain.keywords.is_empty() {
            return 0.0;
        }

        let prompt_lower = prompt.to_lowercase();
        let matched_count = domain
            .keywords
            .iter()
            .filter(|kw| prompt_lower.contains(&kw.to_lowercase()))
            .count();

        matched_count as f32 / domain.keywords.len() as f32
    }

    /// Compute the hybrid score from keyword and embedding components.
    ///
    /// When embedding score is `None` (e.g., AI provider unavailable),
    /// falls back to keyword-only scoring.
    pub fn hybrid_score(&self, keyword_score: f32, embedding_score: Option<f32>) -> f32 {
        match embedding_score {
            Some(emb) => keyword_score * self.keyword_weight + emb * self.embedding_weight,
            None => keyword_score,
        }
    }

    /// Build a `ClassificationResult` from a list of scored domains.
    ///
    /// Ranks domains by hybrid score (descending), selects the top domain,
    /// and returns alternatives with their scores.
    ///
    /// # Errors
    ///
    /// Returns `ClassifyError::NoCandidates` if `domain_scores` is empty.
    pub fn classify_from_scores(
        &self,
        domain_scores: Vec<ScoredDomain>,
    ) -> Result<ClassificationResult, ClassifyError> {
        if domain_scores.is_empty() {
            return Err(ClassifyError::NoCandidates);
        }

        // Compute hybrid scores and sort descending
        let mut ranked: Vec<(String, f32, Vec<String>, f32)> = domain_scores
            .into_iter()
            .map(|sd| {
                let hybrid = self.hybrid_score(sd.keyword_score, sd.embedding_score);
                (sd.domain, hybrid, sd.matched_slots, sd.keyword_score)
            })
            .collect();

        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let top = ranked.remove(0);
        let alternatives: Vec<DomainScore> = ranked
            .into_iter()
            .map(|(domain, confidence, _, _)| DomainScore { domain, confidence })
            .collect();

        Ok(ClassificationResult {
            domain: top.0,
            confidence: top.1,
            matched_slots: top.2,
            alternatives,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_domain(name: &str, keywords: &[&str], slots: &[&str]) -> DomainDefinition {
        DomainDefinition {
            name: name.to_string(),
            keywords: keywords.iter().map(|k| k.to_string()).collect(),
            slots: slots.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn test_keyword_score_full_match() {
        let domain = make_domain("rust_code", &["rust", "cargo", "axum"], &[]);
        let score = HybridClassifier::keyword_score("write a rust axum server with cargo", &domain);
        assert!(
            score > 0.99,
            "All keywords matched, expected ~1.0, got {score}"
        );
    }

    #[test]
    fn test_keyword_score_partial_match() {
        let domain = make_domain("rust_code", &["rust", "cargo", "axum", "tokio"], &[]);
        let score = HybridClassifier::keyword_score("rust and cargo project", &domain);
        assert!(
            (score - 0.5).abs() < 0.01,
            "2 of 4 keywords matched, expected 0.5, got {score}"
        );
    }

    #[test]
    fn test_keyword_score_no_match() {
        let domain = make_domain("sudoku", &["sudoku", "puzzle", "grid", "9x9"], &[]);
        let score = HybridClassifier::keyword_score("write a web server in python", &domain);
        assert_eq!(score, 0.0, "No keywords matched, expected 0.0");
    }

    #[test]
    fn test_keyword_score_case_insensitive() {
        let domain = make_domain("rust_code", &["Rust", "Cargo", "Axum"], &[]);
        let score = HybridClassifier::keyword_score("RUST CARGO AXUM", &domain);
        assert!(
            score > 0.99,
            "Case-insensitive match, expected ~1.0, got {score}"
        );
    }

    #[test]
    fn test_keyword_score_empty_keywords() {
        let domain = make_domain("empty", &[], &[]);
        let score = HybridClassifier::keyword_score("any prompt text", &domain);
        assert_eq!(score, 0.0, "Empty keywords, expected 0.0");
    }

    #[test]
    fn test_hybrid_score_both() {
        let classifier = HybridClassifier::new();
        let score = classifier.hybrid_score(0.5, Some(0.8));
        let expected = 0.5 * 0.3 + 0.8 * 0.7;
        assert!(
            (score - expected).abs() < 0.001,
            "Expected {expected}, got {score}"
        );
    }

    #[test]
    fn test_hybrid_score_keyword_fallback() {
        let classifier = HybridClassifier::new();
        let score = classifier.hybrid_score(0.6, None);
        assert_eq!(score, 0.6, "No embedding, expected keyword score 0.6");
    }

    #[test]
    fn test_classify_from_scores_single() {
        let classifier = HybridClassifier::new();
        let result = classifier
            .classify_from_scores(vec![ScoredDomain {
                domain: "rust_code".to_string(),
                keyword_score: 0.8,
                embedding_score: Some(0.9),
                matched_slots: vec!["apis".to_string()],
            }])
            .unwrap();

        assert_eq!(result.domain, "rust_code");
        assert!(result.confidence > 0.8);
        assert_eq!(result.matched_slots, vec!["apis"]);
        assert!(result.alternatives.is_empty());
    }

    #[test]
    fn test_classify_from_scores_ranked() {
        let classifier = HybridClassifier::new();
        let result = classifier
            .classify_from_scores(vec![
                ScoredDomain {
                    domain: "sudoku".to_string(),
                    keyword_score: 0.2,
                    embedding_score: Some(0.1),
                    matched_slots: vec![],
                },
                ScoredDomain {
                    domain: "py2rs".to_string(),
                    keyword_score: 0.9,
                    embedding_score: Some(0.95),
                    matched_slots: vec!["apis".to_string(), "types".to_string()],
                },
                ScoredDomain {
                    domain: "rust_code".to_string(),
                    keyword_score: 0.5,
                    embedding_score: Some(0.6),
                    matched_slots: vec!["apis".to_string()],
                },
            ])
            .unwrap();

        assert_eq!(result.domain, "py2rs");
        assert!(result.confidence > 0.8);
        assert_eq!(result.matched_slots, vec!["apis", "types"]);
        assert_eq!(result.alternatives.len(), 2);
        // Second best should be rust_code
        assert_eq!(result.alternatives[0].domain, "rust_code");
    }

    #[test]
    fn test_classify_from_scores_empty() {
        let classifier = HybridClassifier::new();
        let result = classifier.classify_from_scores(vec![]);
        assert!(result.is_err());
        match result {
            Err(ClassifyError::NoCandidates) => {}
            _ => panic!("Expected NoCandidates error"),
        }
    }

    #[test]
    fn test_classify_keyword_fallback_wins() {
        let classifier = HybridClassifier::new();
        // Sudoku has great keyword score but no embedding
        // Rust has mediocre both
        let result = classifier
            .classify_from_scores(vec![
                ScoredDomain {
                    domain: "sudoku".to_string(),
                    keyword_score: 1.0,
                    embedding_score: None, // AI unavailable
                    matched_slots: vec![],
                },
                ScoredDomain {
                    domain: "rust_code".to_string(),
                    keyword_score: 0.3,
                    embedding_score: Some(0.3),
                    matched_slots: vec![],
                },
            ])
            .unwrap();

        // keyword-only 1.0 > hybrid 0.3*0.3 + 0.3*0.7 = 0.3
        assert_eq!(result.domain, "sudoku");
        assert!(result.confidence > 0.9);
    }

    #[test]
    fn test_with_custom_weights() {
        let classifier = HybridClassifier::with_weights(0.5, 0.5);
        let score = classifier.hybrid_score(0.6, Some(0.8));
        let expected = 0.6 * 0.5 + 0.8 * 0.5;
        assert!(
            (score - expected).abs() < 0.001,
            "Expected {expected}, got {score}"
        );
    }
}
