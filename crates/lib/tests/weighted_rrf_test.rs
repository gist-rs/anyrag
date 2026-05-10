//! Tests for weighted Reciprocal Rank Fusion (Plan 002: Context Pollution Prevention).

use anyrag::{
    rerank::{reciprocal_rank_fusion, reciprocal_rank_fusion_weighted, RrfWeights},
    types::{QueryContext, SearchResult, SearchSourceType},
};

/// Helper to create a test search result.
fn make_result(title: &str, source_type: SearchSourceType) -> SearchResult {
    SearchResult {
        title: title.to_string(),
        link: format!("http://example.com/{title}"),
        description: format!("Content of {title}"),
        score: 0.0,
        source_type,
    }
}

#[test]
fn test_weighted_rrf_code_generation_boosts_code() {
    // Code result and doc result with same rank position
    let code_result = make_result("Code Example", SearchSourceType::Code);
    let doc_result = make_result("Documentation", SearchSourceType::Documentation);

    let set1 = vec![code_result.clone()];
    let set2 = vec![doc_result.clone()];

    let weights = RrfWeights::from_context(QueryContext::CodeGeneration);
    let results = reciprocal_rank_fusion_weighted(&[set1, set2], &weights);

    assert_eq!(results.len(), 2, "Should have 2 results");
    // Code result should be ranked higher than documentation
    assert_eq!(
        results[0].title, "Code Example",
        "Code should rank above docs in CodeGeneration context"
    );
    assert_eq!(results[1].title, "Documentation");
    // Verify the score gap is significant
    assert!(
        results[0].score > results[1].score * 2.0,
        "Code boost should be significant"
    );
}

#[test]
fn test_weighted_rrf_explanation_balanced() {
    let code_result = make_result("Code Example", SearchSourceType::Code);
    let doc_result = make_result("Documentation", SearchSourceType::Documentation);

    let set1 = vec![code_result.clone()];
    let set2 = vec![doc_result.clone()];

    let weights = RrfWeights::from_context(QueryContext::Explanation);
    let results = reciprocal_rank_fusion_weighted(&[set1, set2], &weights);

    assert_eq!(results.len(), 2);
    // In explanation mode, both should have similar weights (metadata bias is still 100 though)
    // Both are in different sets, so set1 gets metadata weight
    assert_eq!(results[0].title, "Code Example"); // Still first due to metadata bias
}

#[test]
fn test_weighted_rrf_backward_compatible() {
    // The default weights should produce same behavior as old reciprocal_rank_fusion
    let doc_a = SearchResult {
        title: "A".to_string(),
        link: "http://example.com/a".to_string(),
        description: "Content A".to_string(),
        score: 0.0,
        source_type: SearchSourceType::Unknown,
    };
    let doc_b = SearchResult {
        title: "B".to_string(),
        link: "http://example.com/b".to_string(),
        description: "Content B".to_string(),
        score: 0.0,
        source_type: SearchSourceType::Unknown,
    };

    let result_sets = vec![vec![doc_a.clone(), doc_b.clone()]];

    let old_results = reciprocal_rank_fusion(result_sets.clone());
    let new_results = reciprocal_rank_fusion_weighted(&result_sets, &RrfWeights::default());

    // Both should produce the same ordering
    assert_eq!(old_results.len(), new_results.len());
    for (old, new) in old_results.iter().zip(new_results.iter()) {
        assert_eq!(old.title, new.title);
        assert!(
            (old.score - new.score).abs() < 0.0001,
            "Scores should match: {} vs {}",
            old.score,
            new.score
        );
    }
}

#[test]
fn test_rrf_weights_defaults() {
    let weights = RrfWeights::default();
    assert_eq!(weights.metadata, 100.0);
    assert_eq!(weights.vector, 1.0);
    assert_eq!(weights.keyword, 1.0);
    assert_eq!(weights.code_boost, 1.0); // Neutral
    assert_eq!(weights.doc_penalty, 1.0); // Neutral
}

#[test]
fn test_rrf_weights_code_generation() {
    let weights = RrfWeights::from_context(QueryContext::CodeGeneration);
    assert_eq!(weights.code_boost, 10.0);
    assert_eq!(weights.doc_penalty, 0.5);
}

#[test]
fn test_rrf_weights_debugging() {
    let weights = RrfWeights::from_context(QueryContext::Debugging);
    assert_eq!(weights.code_boost, 5.0);
    assert_eq!(weights.doc_penalty, 0.8);
}
