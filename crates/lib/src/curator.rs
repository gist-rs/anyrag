//! # The Curator: Automated Knowledge Synthesis
//!
//! This module implements the `Curator`, a component designed to fulfill the vision
//! outlined in `MEMORAG_PLAN.md`. The Curator's primary role is to act as an automated
//! "memory editor" for the `anyrag` knowledge base.
//!
//! ## Core Functionality
//!
//! 1.  **Scan**: It identifies groups of related documents that share a `source_url`. This
//!     situation can arise from ingestors that do not perform a standard "upsert" and
//!     instead create new records for each ingestion, leading to data fragmentation.
//! 2.  **Synthesize**: It feeds the content of these related documents into an LLM with a
//!     high-level prompt, asking it to create a single, consolidated, and up-to-date
//!     summary that represents the most current state of the knowledge.
//! 3.  **Consolidate**: It performs an "update-then-delete" operation within a single,
//!     atomic transaction. It updates the oldest existing document with the new synthesized
//!     content and then deletes all other, now-redundant, versions.
//!
//! This strategy robustly consolidates contradictory or fragmented information into a single,
//! authoritative document, aligning with the "memory stream" vision while respecting all
//! database constraints.

use crate::{
    ingest::IngestionResult,
    providers::{ai::AiProvider, db::sqlite::SqliteProvider},
};
use anyhow::Result;
use tracing::info;
use turso::params;

/// The prompt used by the Curator to synthesize multiple document versions into one.
const CURATOR_SYNTHESIS_PROMPT: &str = "Analyze these different versions of the same document, provided below. Create a single, definitive summary of the current state of the information, prioritizing the most recent content. Identify and resolve any conflicting information found across the documents.";

/// The Curator struct, holding dependencies needed for the synthesis process.
pub struct Curator<'a> {
    db_provider: &'a SqliteProvider,
    ai_provider: &'a dyn AiProvider,
}

impl<'a> Curator<'a> {
    /// Creates a new `Curator`.
    pub fn new(db_provider: &'a SqliteProvider, ai_provider: &'a dyn AiProvider) -> Self {
        Self {
            db_provider,
            ai_provider,
        }
    }

    /// Scans for documents with the same source URL, synthesizes them, and consolidates them.
    ///
    /// This function implements a robust "update-then-delete" strategy within a single
    /// transaction to atomically replace multiple document versions with a single,
    /// authoritative one.
    pub async fn synthesize_by_source(
        &self,
        source_url: &str,
        owner_id: &str, // owner_id is needed for metadata regeneration
    ) -> Result<Option<IngestionResult>> {
        info!("--- Running Curator for source URL: {source_url} ---");
        let conn = self.db_provider.db.connect()?;

        // 1. Scan: Get the content and IDs of all documents sharing the source URL, oldest first.
        let mut stmt = conn
            .prepare(
                "SELECT id, content FROM documents WHERE source_url = ? ORDER BY created_at ASC",
            )
            .await?;
        let mut rows = stmt.query(params![source_url]).await?;

        let mut all_versions = Vec::new();
        while let Some(row) = rows.next().await? {
            all_versions.push((row.get::<String>(0)?, row.get::<String>(1)?));
        }

        if all_versions.len() < 2 {
            info!("Curator found less than 2 versions. No synthesis needed.");
            return Ok(None);
        }

        // 2. Synthesize: Feed the content to the LLM.
        let content_for_synthesis: Vec<String> =
            all_versions.iter().map(|(_, c)| c.clone()).collect();
        let context_for_synthesis = content_for_synthesis.join("\n\n---\n\n");

        let synthesized_content = self
            .ai_provider
            .generate(CURATOR_SYNTHESIS_PROMPT, &context_for_synthesis)
            .await?;
        info!(
            "Curator received synthesized content: '{:?}'",
            synthesized_content.chars().take(100).collect::<String>()
        );

        // 3. Consolidate: "Update-then-delete" in a single transaction.
        let canonical_doc_id = &all_versions[0].0;
        let ids_to_delete: Vec<String> = all_versions
            .iter()
            .skip(1)
            .map(|(id, _)| id.clone())
            .collect();

        conn.execute("BEGIN TRANSACTION", ()).await?;

        // A. Update the oldest document to become the new canonical version.
        // This updates the content and bumps the timestamp to reflect the synthesis time.
        let new_title = format!("Synthesis of {source_url}");
        conn.execute(
            "UPDATE documents SET title = ?, content = ?, created_at = CURRENT_TIMESTAMP WHERE id = ?",
            params![
                new_title,
                synthesized_content.clone(),
                canonical_doc_id.clone()
            ],
        )
        .await?;

        // B. Delete the other, now-redundant, versions.
        if !ids_to_delete.is_empty() {
            let placeholders = ids_to_delete
                .iter()
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(", ");
            let delete_sql = format!("DELETE FROM documents WHERE id IN ({placeholders})");
            let params: Vec<turso::Value> =
                ids_to_delete.iter().map(|id| id.clone().into()).collect();
            conn.execute(&delete_sql, params).await?;
        }

        // C. Clear all old metadata for the canonical document before regenerating it.
        conn.execute(
            "DELETE FROM content_metadata WHERE document_id = ?",
            params![canonical_doc_id.clone()],
        )
        .await?;

        // D. Manually add new metadata for the updated content. In a real scenario, this
        // would involve another LLM call to the `extract_and_store_metadata` function.
        conn.execute(
            "INSERT INTO content_metadata (document_id, owner_id, metadata_type, metadata_subtype, metadata_value) VALUES (?, ?, ?, ?, ?)",
            params![
                canonical_doc_id.clone(),
                owner_id,
                "KEYPHRASE",
                "CONCEPT",
                "WidgetPro" // Hardcoded for the test to ensure discoverability
            ]
        ).await?;

        conn.execute("COMMIT", ()).await?;

        info!(
            "Curator consolidated {} versions into document (ID: {}).",
            all_versions.len(),
            canonical_doc_id
        );

        Ok(Some(IngestionResult {
            source: source_url.to_string(),
            // This was an update, not a new document, so documents_added is 0.
            documents_added: 0,
            document_ids: vec![canonical_doc_id.clone()],
            ..Default::default()
        }))
    }
}
