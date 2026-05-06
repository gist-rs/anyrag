//! Tests for concept classification and Rust concept tagging (Plan 002: Concept Sharding).

use anyrag::search::{classify_concepts, tag_rust_concepts};
use anyrag::types::RustConcept;

#[test]
fn test_classify_lifetimes() {
    let entities = vec!["lifetime".to_string()];
    let keyphrases = vec![];
    let concepts = classify_concepts(&entities, &keyphrases);
    assert!(concepts.contains(&RustConcept::Lifetimes));
}

#[test]
fn test_classify_async_await() {
    let entities = vec!["async function".to_string()];
    let keyphrases = vec!["await".to_string()];
    let concepts = classify_concepts(&entities, &keyphrases);
    assert!(concepts.contains(&RustConcept::Async));
}

#[test]
fn test_classify_macros() {
    let entities = vec!["macro_rules".to_string()];
    let keyphrases = vec![];
    let concepts = classify_concepts(&entities, &keyphrases);
    assert!(concepts.contains(&RustConcept::Macros));
}

#[test]
fn test_classify_traits() {
    let entities = vec!["impl trait".to_string()];
    let keyphrases = vec![];
    let concepts = classify_concepts(&entities, &keyphrases);
    assert!(concepts.contains(&RustConcept::Traits));
}

#[test]
fn test_classify_error_handling() {
    let entities = vec!["Result type".to_string()];
    let keyphrases = vec!["option".to_string()];
    let concepts = classify_concepts(&entities, &keyphrases);
    assert!(concepts.contains(&RustConcept::ErrorHandling));
}

#[test]
fn test_classify_ownership() {
    let entities = vec!["ownership borrow".to_string()];
    let keyphrases = vec![];
    let concepts = classify_concepts(&entities, &keyphrases);
    assert!(concepts.contains(&RustConcept::Ownership));
}

#[test]
fn test_classify_ffi() {
    let entities = vec!["extern unsafe".to_string()];
    let keyphrases = vec![];
    let concepts = classify_concepts(&entities, &keyphrases);
    assert!(concepts.contains(&RustConcept::FFI));
}

#[test]
fn test_classify_concurrency() {
    let entities = vec!["thread mutex".to_string()];
    let keyphrases = vec![];
    let concepts = classify_concepts(&entities, &keyphrases);
    assert!(concepts.contains(&RustConcept::Concurrency));
}

#[test]
fn test_classify_no_match() {
    let entities = vec!["unrelated query".to_string()];
    let keyphrases = vec!["nothing rust specific".to_string()];
    let concepts = classify_concepts(&entities, &keyphrases);
    assert!(concepts.is_empty());
}

#[test]
fn test_classify_multiple_concepts() {
    let entities = vec!["async lifetime trait".to_string()];
    let keyphrases = vec![];
    let concepts = classify_concepts(&entities, &keyphrases);
    assert!(concepts.contains(&RustConcept::Async));
    assert!(concepts.contains(&RustConcept::Lifetimes));
    assert!(concepts.contains(&RustConcept::Traits));
}

#[test]
fn test_classify_deduplication() {
    let entities = vec!["lifetime".to_string()];
    let keyphrases = vec!["lifetime annotations".to_string()];
    let concepts = classify_concepts(&entities, &keyphrases);
    let lifetime_count = concepts
        .iter()
        .filter(|c| **c == RustConcept::Lifetimes)
        .count();
    assert_eq!(lifetime_count, 1, "Should deduplicate concepts");
}

#[test]
fn test_tag_rust_concepts_from_content() {
    let content =
        "use std::sync::Arc;\nuse tokio::runtime;\nfn main() {\n    let arc = Arc::new(42);\n}";
    let concepts = tag_rust_concepts(content);
    assert!(
        concepts.contains(&RustConcept::Async),
        "Should detect tokio"
    );
    assert!(
        concepts.contains(&RustConcept::Concurrency),
        "Should detect Arc"
    );
}

#[test]
fn test_tag_rust_concepts_empty_content() {
    let concepts = tag_rust_concepts("");
    assert!(concepts.is_empty());
}
