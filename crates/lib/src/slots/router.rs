//! # Keyword Router
//!
//! Deterministic keyword-based router for assigning documents to slots.
//! No LLM, no neural net — pure string matching against slot keyword lists.

use std::collections::HashMap;

use super::types::{RouteMethod, RouteResult, Slot, SlotDocument, SlotName};

/// Deterministic keyword-based router. No LLM, no neural net.
/// Matches document content against slot keyword lists.
pub struct KeywordRouter {
    slot_definitions: Vec<Slot>,
}

impl KeywordRouter {
    /// Create a new router with the given slot definitions.
    pub fn new(slot_definitions: Vec<Slot>) -> Self {
        Self { slot_definitions }
    }

    /// Route a document to matching slots based on content analysis.
    /// Returns all matching slots (a document can live in multiple slots).
    pub fn route(&self, content: &str, document_id: &str) -> RouteResult {
        let content_lower = content.to_lowercase();
        let mut assigned_slots = Vec::new();
        let mut matched_keywords = HashMap::new();

        for slot in &self.slot_definitions {
            let matches: Vec<String> = slot
                .keywords
                .iter()
                .filter(|kw| content_lower.contains(&kw.to_lowercase()))
                .cloned()
                .collect();

            if !matches.is_empty() {
                assigned_slots.push(slot.name.clone());
                matched_keywords.insert(slot.name.clone(), matches);
            }
        }

        RouteResult {
            document_id: document_id.to_string(),
            assigned_slots,
            matched_keywords,
        }
    }

    /// Convert a `RouteResult` into `SlotDocument` rows for persistence.
    /// Each assigned slot becomes one `SlotDocument` with initial relevance 1.0.
    pub fn result_to_slot_documents(&self, result: &RouteResult) -> Vec<SlotDocument> {
        let now = chrono::Utc::now().to_rfc3339();

        result
            .assigned_slots
            .iter()
            .map(|slot_name| SlotDocument {
                id: uuid::Uuid::now_v7().to_string(),
                slot_name: slot_name.clone(),
                document_id: result.document_id.clone(),
                routed_by: RouteMethod::Keyword,
                routed_at: now.clone(),
                relevance_score: 1.0,
            })
            .collect()
    }
}

/// Default keyword rules for code RAG slots.
/// These are intentionally conservative — false negatives are better than false positives.
pub fn default_slot_keywords() -> HashMap<SlotName, Vec<String>> {
    let mut map = HashMap::new();
    map.insert(
        SlotName::Architecture,
        vec![
            "mod.rs".to_string(),
            "lib.rs".to_string(),
            "main.rs".to_string(),
            "cargo.toml".to_string(),
            "architecture".to_string(),
            "module".to_string(),
            "crate".to_string(),
            "workspace".to_string(),
            "design".to_string(),
            "overview".to_string(),
            "structure".to_string(),
            "diagram".to_string(),
            "layout".to_string(),
        ],
    );
    map.insert(
        SlotName::Types,
        vec![
            "struct ".to_string(),
            "enum ".to_string(),
            "type ".to_string(),
            "impl ".to_string(),
            "trait ".to_string(),
            "pub struct".to_string(),
            "pub enum".to_string(),
            "type alias".to_string(),
            "newtype".to_string(),
        ],
    );
    map.insert(
        SlotName::Apis,
        vec![
            "pub fn ".to_string(),
            "pub async fn ".to_string(),
            "pub trait ".to_string(),
            "pub mod ".to_string(),
            "#[export_name".to_string(),
            "no_mangle".to_string(),
            "extern \"c\"".to_string(),
            "api".to_string(),
            "endpoint".to_string(),
            "handler".to_string(),
            "route".to_string(),
        ],
    );
    map.insert(
        SlotName::Dependencies,
        vec![
            "[dependencies]".to_string(),
            "[dev-dependencies]".to_string(),
            "cargo.toml".to_string(),
            "crate =".to_string(),
            "version =".to_string(),
            "features =".to_string(),
            "path =".to_string(),
        ],
    );
    map.insert(
        SlotName::Tests,
        vec![
            "#[test]".to_string(),
            "#[tokio::test]".to_string(),
            "mod tests".to_string(),
            "fn test_".to_string(),
            "assert!".to_string(),
            "assert_eq!".to_string(),
            "assert_ne!".to_string(),
            "#[bench]".to_string(),
            "bench".to_string(),
            "criterion".to_string(),
            "proptest".to_string(),
        ],
    );
    map.insert(
        SlotName::Chatter,
        vec![
            "todo".to_string(),
            "fixme".to_string(),
            "hack".to_string(),
            "xxx".to_string(),
            "note:".to_string(),
            "changelog".to_string(),
            "contributing".to_string(),
            "readme".to_string(),
        ],
    );
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_router() -> KeywordRouter {
        let keywords = default_slot_keywords();
        let slots: Vec<Slot> = keywords
            .into_iter()
            .map(|(name, keywords)| Slot {
                id: uuid::Uuid::now_v7().to_string(),
                name,
                description: String::new(),
                is_frozen: false,
                decay_rate: 0.1,
                max_documents: 1000,
                keywords,
                created_at: String::new(),
                updated_at: String::new(),
            })
            .collect();
        KeywordRouter::new(slots)
    }

    #[test]
    fn test_architecture_matches_mod_rs() {
        let router = test_router();
        let result = router.route("this is mod.rs content", "doc-1");
        assert!(result.assigned_slots.contains(&SlotName::Architecture));
        assert!(result
            .matched_keywords
            .get(&SlotName::Architecture)
            .unwrap()
            .contains(&"mod.rs".to_string()));
    }

    #[test]
    fn test_single_document_routes_to_multiple_slots() {
        let router = test_router();
        // A file with both struct definition and pub fn — should match types + apis
        let content = "pub struct Foo {}\npub fn bar() -> i32 { 42 }\nimpl Foo { pub fn new() -> Self { Self {} } }";
        let result = router.route(content, "doc-2");
        assert!(
            result.assigned_slots.contains(&SlotName::Types),
            "Should route to types: {:?}",
            result.assigned_slots
        );
        assert!(
            result.assigned_slots.contains(&SlotName::Apis),
            "Should route to apis: {:?}",
            result.assigned_slots
        );
    }

    #[test]
    fn test_no_match_returns_empty() {
        let router = test_router();
        let result = router.route("just some random text with no keywords", "doc-3");
        assert!(result.assigned_slots.is_empty());
        assert!(result.matched_keywords.is_empty());
    }

    #[test]
    fn test_test_slot_matches() {
        let router = test_router();
        let content = "#[test]\nfn test_foo() {\n    assert_eq!(1 + 1, 2);\n}";
        let result = router.route(content, "doc-4");
        assert!(result.assigned_slots.contains(&SlotName::Tests));
    }

    #[test]
    fn test_dependencies_slot_matches() {
        let router = test_router();
        let content = "[dependencies]\nserde = { version = \"1.0\", features = [\"derive\"] }";
        let result = router.route(content, "doc-5");
        assert!(result.assigned_slots.contains(&SlotName::Dependencies));
    }

    #[test]
    fn test_case_insensitive_matching() {
        let router = test_router();
        let result = router.route("ARCHITECTURE overview document", "doc-6");
        assert!(result.assigned_slots.contains(&SlotName::Architecture));
    }

    #[test]
    fn test_result_to_slot_documents() {
        let router = test_router();
        let result = router.route("pub struct Foo {}\npub fn bar() {}", "doc-7");
        let docs = router.result_to_slot_documents(&result);

        assert_eq!(docs.len(), result.assigned_slots.len());
        for doc in &docs {
            assert_eq!(doc.document_id, "doc-7");
            assert_eq!(doc.routed_by, RouteMethod::Keyword);
            assert_eq!(doc.relevance_score, 1.0);
        }
    }
}
