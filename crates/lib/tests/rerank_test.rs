//! # Rerank Logic Tests
//!
//! This file contains tests for the `reciprocal_rank_fusion` function
//! to ensure its logic for de-duplication and versioning is correct.

use anyrag::{rerank::reciprocal_rank_fusion, types::SearchResult};

#[test]
fn test_rrf_deduplication_and_versioning() {
    // --- 1. Arrange ---

    // A common document that should be de-duplicated.
    let doc_a = SearchResult {
        title: "Document A".to_string(),
        link: "http://example.com/doc_a".to_string(),
        description: "Content of A".to_string(),
        score: 0.0, // Initial score doesn't matter
    };

    // A unique document.
    let doc_b = SearchResult {
        title: "Document B".to_string(),
        link: "http://example.com/doc_b".to_string(),
        description: "Content of B".to_string(),
        score: 0.0,
    };

    // Two versions of the same document (same link, different content).
    // These should NOT be de-duplicated.
    let doc_c_v1 = SearchResult {
        title: "Document C v1".to_string(),
        link: "http://example.com/doc_c".to_string(),
        description: "Content of C, version 1".to_string(),
        score: 0.0,
    };
    let doc_c_v2 = SearchResult {
        title: "Document C v2".to_string(),
        link: "http://example.com/doc_c".to_string(),
        description: "Content of C, version 2".to_string(),
        score: 0.0,
    };

    // Create two result sets.
    // Set 1 is the high-priority metadata search.
    let set1 = vec![doc_c_v1.clone(), doc_a.clone(), doc_b.clone()];
    // Set 2 is a lower-priority keyword search.
    let set2 = vec![doc_a.clone(), doc_c_v2.clone()];

    // --- 2. Act ---
    let final_results = reciprocal_rank_fusion(vec![set1, set2]);

    // --- 3. Assert ---

    // A. Assert correct count (de-duplication and version preservation)
    assert_eq!(
        final_results.len(),
        4,
        "Expected 4 unique results: A, B, C_v1, C_v2. doc_A should have been de-duplicated."
    );

    // B. Assert correct ranking order
    let titles: Vec<String> = final_results.iter().map(|r| r.title.clone()).collect();

    // Expected order based on RRF scores:
    // doc_c_v1 (set1, rank 0): score = 100/1 = 100
    // doc_a (set1, rank 1; set2, rank 0): score = 100/2 + 1/61 = 50.016
    // doc_b (set1, rank 2): score = 100/3 = 33.33
    // doc_c_v2 (set2, rank 1): score = 1/62 = 0.016
    // So, order should be: C_v1, A, B, C_v2
    let expected_titles = vec![
        "Document C v1".to_string(),
        "Document A".to_string(),
        "Document B".to_string(),
        "Document C v2".to_string(),
    ];

    assert_eq!(
        titles, expected_titles,
        "Results are not in the correct ranked order."
    );
}
