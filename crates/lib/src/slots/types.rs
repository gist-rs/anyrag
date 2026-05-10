//! # Slot Types
//!
//! Core types for the Raven Routed Slot Memory system.
//! Slots are named partitions that categorize ingested documents
//! for deterministic retrieval and selective decay.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Named slot for categorizing ingested documents.
/// Maps to Raven's "slot" concept — a bounded memory partition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Slot {
    pub id: String,     // UUID v7
    pub name: SlotName, // e.g. "architecture", "types", "apis"
    pub description: String,
    pub is_frozen: bool, // frozen slots never decay (Raven: unselected = preserved)
    pub decay_rate: f64, // Raven Equation 18: λ (lambda), 0.0 = no decay
    pub max_documents: usize, // soft cap per slot
    pub keywords: Vec<String>, // routing keywords (deterministic matching)
    pub created_at: String,
    pub updated_at: String,
}

/// Default slot names for code RAG workloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SlotName {
    /// System design, module structure, high-level patterns. FROZEN — never decays.
    Architecture,
    /// Type definitions, structs, enums, type aliases.
    Types,
    /// Public API surfaces, function signatures, trait definitions.
    Apis,
    /// Crate dependencies, version constraints, feature flags.
    Dependencies,
    /// Test files, test utilities, benchmark harnesses.
    Tests,
    /// Conversational context, chat logs, informal notes. High decay.
    Chatter,
    /// User-defined custom slot.
    Custom(String),
}

impl std::fmt::Display for SlotName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SlotName::Architecture => write!(f, "architecture"),
            SlotName::Types => write!(f, "types"),
            SlotName::Apis => write!(f, "apis"),
            SlotName::Dependencies => write!(f, "dependencies"),
            SlotName::Tests => write!(f, "tests"),
            SlotName::Chatter => write!(f, "chatter"),
            SlotName::Custom(name) => write!(f, "{name}"),
        }
    }
}

/// Association between a document and a slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotDocument {
    pub id: String, // UUID v7
    pub slot_name: SlotName,
    pub document_id: String, // FK to documents table
    pub routed_by: RouteMethod,
    pub routed_at: String,    // ISO timestamp
    pub relevance_score: f64, // initial score, decays over time
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteMethod {
    /// Phase 1: deterministic keyword matching.
    Keyword,
    /// Phase 2 (future): embedding-based neural routing.
    Neural,
}

impl std::fmt::Display for RouteMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouteMethod::Keyword => write!(f, "keyword"),
            RouteMethod::Neural => write!(f, "neural"),
        }
    }
}

/// Result of routing a document to slots.
#[derive(Debug, Clone)]
pub struct RouteResult {
    pub document_id: String,
    pub assigned_slots: Vec<SlotName>,
    pub matched_keywords: HashMap<SlotName, Vec<String>>,
}
