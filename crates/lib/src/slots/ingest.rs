//! # Slot-Aware Ingestion Hook
//!
//! Additive layer over existing ingestion that routes documents to slots
//! after insertion. This is opt-in — existing ingestion is untouched.
//!
//! Usage:
//! ```ignore
//! let ingester = SlotIngester::new(db);
//! ingester.ensure_slots().await?;
//! ingester.route_and_persist("doc-id", content).await?;
//! ```

use std::sync::Arc;

use turso::Database;

use crate::errors::PromptError;
use crate::providers::db::sqlite::sql;

use super::decay;
use super::router::KeywordRouter;
use super::seeder::seed_default_slots;
use super::types::{Slot, SlotDocument, SlotName};

/// Slot-aware ingestion hook.
///
/// After a document is inserted via the existing pipeline, this struct
/// can route the document to matching slots based on keyword analysis.
/// It is additive — existing ingestion is completely untouched.
pub struct SlotIngester {
    db: Arc<Database>,
}

impl SlotIngester {
    /// Create a new `SlotIngester` wrapping the given database.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Ensure the slot schema and default slots exist.
    /// Idempotent — safe to call on every startup.
    pub async fn ensure_schema(&self) -> Result<(), PromptError> {
        let conn = self
            .db
            .connect()
            .map_err(|e| PromptError::StorageConnection(e.to_string()))?;

        // Create tables
        for statement in sql::ALL_TABLE_CREATION_SQL {
            conn.execute(statement, ())
                .await
                .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?;
        }

        Ok(())
    }

    /// Ensure default slots are seeded. Idempotent — only inserts if not present.
    pub async fn ensure_default_slots(&self) -> Result<(), PromptError> {
        let conn = self
            .db
            .connect()
            .map_err(|e| PromptError::StorageConnection(e.to_string()))?;

        let defaults = seed_default_slots();

        for slot in &defaults {
            // Only insert if not already present (by name as PK)
            let params: Vec<turso::Value> = vec![slot.name.to_string().into()];
            let mut rows = conn
                .query(
                    "SELECT COUNT(*) as cnt FROM rag_slots WHERE name = ?1",
                    params,
                )
                .await
                .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?;

            let count = if let Some(row) = rows
                .next()
                .await
                .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?
            {
                row.get::<i64>(0).unwrap_or(0)
            } else {
                0
            };

            if count == 0 {
                self.insert_slot(&conn, slot).await?;
            }
        }

        Ok(())
    }

    /// Insert a single slot into the database.
    async fn insert_slot(&self, conn: &turso::Connection, slot: &Slot) -> Result<(), PromptError> {
        let keywords_json =
            serde_json::to_string(&slot.keywords).map_err(PromptError::JsonSerialization)?;

        let params: Vec<turso::Value> = vec![
            slot.name.to_string().into(),
            slot.description.clone().into(),
            (if slot.is_frozen { 1 } else { 0 }).into(),
            slot.decay_rate.into(),
            (slot.max_documents as i64).into(),
            keywords_json.into(),
            slot.created_at.clone().into(),
            slot.updated_at.clone().into(),
        ];

        conn.execute(
            "INSERT INTO rag_slots (name, description, is_frozen, decay_rate, max_documents, keywords, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params,
        )
        .await
        .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?;

        Ok(())
    }

    /// Load all slot definitions from the database.
    pub async fn load_slots(&self) -> Result<Vec<Slot>, PromptError> {
        let conn = self
            .db
            .connect()
            .map_err(|e| PromptError::StorageConnection(e.to_string()))?;

        let mut result = conn
            .query("SELECT name, description, is_frozen, decay_rate, max_documents, keywords, created_at, updated_at FROM rag_slots", ())
            .await
            .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?;

        let mut slots = Vec::new();

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?
        {
            let name_str: String = row.get(0).unwrap_or_default();
            let description: String = row.get(1).unwrap_or_default();
            let is_frozen: i64 = row.get(2).unwrap_or(0);
            let decay_rate: f64 = row.get(3).unwrap_or(0.1);
            let max_documents: i64 = row.get(4).unwrap_or(1000);
            let keywords_json: String = row.get(5).unwrap_or("[]".to_string());
            let created_at: String = row.get(6).unwrap_or_default();
            let updated_at: String = row.get(7).unwrap_or_default();

            let slot_name = parse_slot_name(&name_str);
            let keywords: Vec<String> = serde_json::from_str(&keywords_json).unwrap_or_default();

            slots.push(Slot {
                id: String::new(), // Not stored in DB, generated
                name: slot_name,
                description,
                is_frozen: is_frozen != 0,
                decay_rate,
                max_documents: max_documents as usize,
                keywords,
                created_at,
                updated_at,
            });
        }

        Ok(slots)
    }

    /// Route a document to matching slots and persist the associations.
    ///
    /// This should be called AFTER the document has been inserted into the
    /// `documents` table. It analyzes the content and creates `slot_documents`
    /// rows for each matching slot.
    ///
    /// Returns the list of `SlotDocument` associations created.
    pub async fn route_and_persist(
        &self,
        document_id: &str,
        content: &str,
    ) -> Result<Vec<SlotDocument>, PromptError> {
        let slots = self.load_slots().await?;
        let router = KeywordRouter::new(slots);
        let route_result = router.route(content, document_id);

        if route_result.assigned_slots.is_empty() {
            return Ok(Vec::new());
        }

        let slot_docs = router.result_to_slot_documents(&route_result);
        let conn = self
            .db
            .connect()
            .map_err(|e| PromptError::StorageConnection(e.to_string()))?;

        for doc in &slot_docs {
            let params: Vec<turso::Value> = vec![
                doc.id.clone().into(),
                doc.slot_name.to_string().into(),
                doc.document_id.clone().into(),
                doc.routed_by.to_string().into(),
                doc.routed_at.clone().into(),
                doc.relevance_score.into(),
            ];

            conn.execute(
                "INSERT OR IGNORE INTO slot_documents (id, slot_name, document_id, routed_by, routed_at, relevance_score) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params,
            )
            .await
            .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?;
        }

        Ok(slot_docs)
    }

    /// Apply decay to all non-frozen slot documents.
    /// Should be called periodically (e.g., daily) to keep scores current.
    pub async fn apply_decay_batch(&self) -> Result<usize, PromptError> {
        let conn = self
            .db
            .connect()
            .map_err(|e| PromptError::StorageConnection(e.to_string()))?;

        conn.execute(decay::apply_decay_batch_sql(), ())
            .await
            .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?;

        // Clean up documents that have decayed below threshold
        let cleanup_sql = decay::cleanup_decayed_sql();
        conn.execute(&cleanup_sql, ())
            .await
            .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?;

        Ok(0) // TODO: return actual affected count if turso supports it
    }

    /// Re-route all existing documents through the keyword router.
    /// Useful after changing slot keyword definitions.
    pub async fn reindex_all(&self) -> Result<ReindexResult, PromptError> {
        let conn = self
            .db
            .connect()
            .map_err(|e| PromptError::StorageConnection(e.to_string()))?;

        // Load all documents
        let mut result = conn
            .query("SELECT id, content FROM documents", ())
            .await
            .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?;

        let mut documents = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?
        {
            let id: String = row.get(0).unwrap_or_default();
            let content: String = row.get(1).unwrap_or_default();
            documents.push((id, content));
        }

        // Clear existing slot_documents
        conn.execute("DELETE FROM slot_documents", ())
            .await
            .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?;

        // Re-route each document
        let mut total_documents = 0;
        let mut total_routing_entries = 0;

        for (doc_id, content) in &documents {
            let slot_docs = self.route_and_persist(doc_id, content).await?;
            if !slot_docs.is_empty() {
                total_documents += 1;
                total_routing_entries += slot_docs.len();
            }
        }

        Ok(ReindexResult {
            total_documents_routed: total_documents,
            total_routing_entries,
            total_documents_scanned: documents.len(),
        })
    }

    /// Create a custom slot with the given configuration.
    pub async fn create_custom_slot(
        &self,
        name: &str,
        description: &str,
        decay_rate: f64,
        keywords: Vec<String>,
    ) -> Result<Slot, PromptError> {
        let now = chrono::Utc::now().to_rfc3339();
        let slot = Slot {
            id: uuid::Uuid::now_v7().to_string(),
            name: SlotName::Custom(name.to_string()),
            description: description.to_string(),
            is_frozen: false,
            decay_rate,
            max_documents: 1000,
            keywords,
            created_at: now.clone(),
            updated_at: now,
        };

        let conn = self
            .db
            .connect()
            .map_err(|e| PromptError::StorageConnection(e.to_string()))?;

        self.insert_slot(&conn, &slot).await?;

        Ok(slot)
    }

    /// Remove a document from a specific slot.
    pub async fn remove_document_from_slot(
        &self,
        slot_name: &str,
        document_id: &str,
    ) -> Result<bool, PromptError> {
        let conn = self
            .db
            .connect()
            .map_err(|e| PromptError::StorageConnection(e.to_string()))?;

        let params: Vec<turso::Value> = vec![slot_name.into(), document_id.into()];

        conn.execute(super::search::remove_document_from_slot_sql(), params)
            .await
            .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?;

        Ok(true)
    }
}

/// Result of reindexing all documents.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReindexResult {
    /// Number of documents that were routed to at least one slot.
    pub total_documents_routed: usize,
    /// Total number of slot_documents entries created.
    pub total_routing_entries: usize,
    /// Total number of documents scanned.
    pub total_documents_scanned: usize,
}

/// Parse a slot name string back into a `SlotName` enum.
fn parse_slot_name(name: &str) -> SlotName {
    match name {
        "architecture" => SlotName::Architecture,
        "types" => SlotName::Types,
        "apis" => SlotName::Apis,
        "dependencies" => SlotName::Dependencies,
        "tests" => SlotName::Tests,
        "chatter" => SlotName::Chatter,
        other => SlotName::Custom(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_slot_name_known() {
        assert_eq!(parse_slot_name("architecture"), SlotName::Architecture);
        assert_eq!(parse_slot_name("types"), SlotName::Types);
        assert_eq!(parse_slot_name("apis"), SlotName::Apis);
        assert_eq!(parse_slot_name("dependencies"), SlotName::Dependencies);
        assert_eq!(parse_slot_name("tests"), SlotName::Tests);
        assert_eq!(parse_slot_name("chatter"), SlotName::Chatter);
    }

    #[test]
    fn test_parse_slot_name_custom() {
        let custom = parse_slot_name("my_custom_slot");
        assert!(matches!(custom, SlotName::Custom(name) if name == "my_custom_slot"));
    }

    #[test]
    fn test_reindex_result_serialization() {
        let result = ReindexResult {
            total_documents_routed: 10,
            total_routing_entries: 25,
            total_documents_scanned: 50,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: ReindexResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total_documents_routed, 10);
        assert_eq!(parsed.total_routing_entries, 25);
        assert_eq!(parsed.total_documents_scanned, 50);
    }
}
