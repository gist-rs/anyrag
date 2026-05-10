//! # Raven Routed Slot Memory
//!
//! Deterministic slot-based memory for code RAG. Documents are assigned to named
//! "slots" (e.g., `architecture`, `types`, `apis`) via keyword matching during
//! ingestion. Search can target active slots only, reducing context pollution.
//!
//! ## Key Concepts
//!
//! - **Slots**: Named partitions for categorizing documents (bounded memory)
//! - **Keyword Router**: Deterministic content matching — no LLM, no neural net
//! - **Selective Decay** (Raven Equation 18): Non-frozen slots decay over time
//! - **Frozen Slots**: Critical slots (e.g., `architecture`) never decay
//!
//! ## Usage
//!
//! ```ignore
//! use anyrag::slots::{SlotIngester, KeywordRouter};
//!
//! // Ensure schema and seed defaults
//! let ingester = SlotIngester::new(db);
//! ingester.ensure_schema().await?;
//! ingester.ensure_default_slots().await?;
//!
//! // Route a document after ingestion
//! let slot_docs = ingester.route_and_persist("doc-id", content).await?;
//! ```

pub mod decay;
pub mod ingest;
pub mod router;
pub mod search;
pub mod seeder;
pub mod types;

pub use decay::{decayed_score, decayed_score_after_days, is_decayed_out, MIN_RELEVANCE_SCORE};
pub use ingest::{ReindexResult, SlotIngester};
pub use router::{default_slot_keywords, KeywordRouter};
pub use search::{build_slot_filter, slot_filtered_document_sql, SlotFilter, SlotSearchConfig};
pub use seeder::seed_default_slots;
pub use types::{RouteMethod, RouteResult, Slot, SlotDocument, SlotName};
