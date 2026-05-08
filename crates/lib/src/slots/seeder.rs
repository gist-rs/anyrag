//! # Slot Seeder
//!
//! Seeds default slots for code RAG workloads with appropriate decay rates.
//! Frozen slots (architecture) never decay; chatter slots decay fastest.

use super::router::default_slot_keywords;
use super::types::{Slot, SlotName};

/// Default decay rates per slot type, following Raven Equation 18.
/// λ (lambda): 0.0 = no decay, higher = faster decay.
const DECAY_ARCHITECTURE: f64 = 0.0;
const DECAY_TYPES: f64 = 0.05;
const DECAY_APIS: f64 = 0.05;
const DECAY_DEPENDENCIES: f64 = 0.1;
const DECAY_TESTS: f64 = 0.1;
const DECAY_CHATTER: f64 = 0.5;

const MAX_DOCUMENTS_DEFAULT: usize = 1000;

/// Creates the default set of slots for a code RAG system.
/// Returns `Slot` instances ready to be inserted into `rag_slots` table.
pub fn seed_default_slots() -> Vec<Slot> {
    let keywords = default_slot_keywords();
    let now = chrono::Utc::now().to_rfc3339();

    vec![
        make_slot(
            SlotName::Architecture,
            "System design, module structure, high-level patterns. FROZEN — never decays.",
            true,
            DECAY_ARCHITECTURE,
            keywords.get(&SlotName::Architecture),
            &now,
        ),
        make_slot(
            SlotName::Types,
            "Type definitions, structs, enums, type aliases.",
            false,
            DECAY_TYPES,
            keywords.get(&SlotName::Types),
            &now,
        ),
        make_slot(
            SlotName::Apis,
            "Public API surfaces, function signatures, trait definitions.",
            false,
            DECAY_APIS,
            keywords.get(&SlotName::Apis),
            &now,
        ),
        make_slot(
            SlotName::Dependencies,
            "Crate dependencies, version constraints, feature flags.",
            false,
            DECAY_DEPENDENCIES,
            keywords.get(&SlotName::Dependencies),
            &now,
        ),
        make_slot(
            SlotName::Tests,
            "Test files, test utilities, benchmark harnesses.",
            false,
            DECAY_TESTS,
            keywords.get(&SlotName::Tests),
            &now,
        ),
        make_slot(
            SlotName::Chatter,
            "Conversational context, chat logs, informal notes. High decay.",
            false,
            DECAY_CHATTER,
            keywords.get(&SlotName::Chatter),
            &now,
        ),
    ]
}

fn make_slot(
    name: SlotName,
    description: &'static str,
    is_frozen: bool,
    decay_rate: f64,
    keywords: Option<&Vec<String>>,
    now: &str,
) -> Slot {
    Slot {
        id: uuid::Uuid::now_v7().to_string(),
        name,
        description: description.to_string(),
        is_frozen,
        decay_rate,
        max_documents: MAX_DOCUMENTS_DEFAULT,
        keywords: keywords.cloned().unwrap_or_default(),
        created_at: now.to_string(),
        updated_at: now.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_default_slots_count() {
        let slots = seed_default_slots();
        // 6 built-in slots: architecture, types, apis, dependencies, tests, chatter
        assert_eq!(slots.len(), 6);
    }

    #[test]
    fn test_architecture_slot_is_frozen() {
        let slots = seed_default_slots();
        let arch = slots
            .iter()
            .find(|s| s.name == SlotName::Architecture)
            .unwrap();
        assert!(arch.is_frozen);
        assert_eq!(arch.decay_rate, 0.0);
    }

    #[test]
    fn test_chatter_slot_has_highest_decay() {
        let slots = seed_default_slots();
        let chatter = slots.iter().find(|s| s.name == SlotName::Chatter).unwrap();
        assert!(!chatter.is_frozen);
        assert_eq!(chatter.decay_rate, 0.5);

        // Verify chatter decay is higher than all non-frozen slots
        for slot in &slots {
            if slot.name != SlotName::Chatter {
                assert!(
                    chatter.decay_rate >= slot.decay_rate,
                    "chatter decay ({}) should be >= {} decay ({})",
                    chatter.decay_rate,
                    slot.name,
                    slot.decay_rate
                );
            }
        }
    }

    #[test]
    fn test_all_slots_have_keywords() {
        let slots = seed_default_slots();
        for slot in &slots {
            assert!(
                !slot.keywords.is_empty(),
                "slot {:?} should have keywords",
                slot.name
            );
        }
    }

    #[test]
    fn test_all_slots_have_unique_ids() {
        let slots = seed_default_slots();
        let ids: Vec<&str> = slots.iter().map(|s| s.id.as_str()).collect();
        let unique_ids: std::collections::HashSet<&str> = ids.iter().copied().collect();
        assert_eq!(ids.len(), unique_ids.len(), "all slot IDs should be unique");
    }

    #[test]
    fn test_slot_names_are_unique() {
        let slots = seed_default_slots();
        let names: Vec<String> = slots.iter().map(|s| s.name.to_string()).collect();
        let unique_names: std::collections::HashSet<String> = names.into_iter().collect();
        assert_eq!(
            slots.len(),
            unique_names.len(),
            "all slot names should be unique"
        );
    }

    #[test]
    fn test_types_and_apis_have_same_decay() {
        let slots = seed_default_slots();
        let types_slot = slots.iter().find(|s| s.name == SlotName::Types).unwrap();
        let apis_slot = slots.iter().find(|s| s.name == SlotName::Apis).unwrap();
        assert_eq!(types_slot.decay_rate, apis_slot.decay_rate);
        assert_eq!(types_slot.decay_rate, 0.05);
    }
}
