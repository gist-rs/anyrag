//! # Routed Slot Search
//!
//! Slot-aware search: only retrieve from active slots.
//! Wraps the existing search pipeline, adding a slot_documents JOIN filter.
//! Frozen slots are always included regardless of active_slots.

use super::types::SlotName;

/// Configuration for slot-filtered search.
///
/// When used, only documents belonging to the specified active slots
/// OR any frozen slot will be included in search results.
pub struct SlotSearchConfig<'a> {
    /// Slots the user explicitly wants to search in.
    pub active_slots: &'a [SlotName],
    /// If true, frozen slots (e.g., architecture) are always included.
    pub include_frozen: bool,
}

impl Default for SlotSearchConfig<'_> {
    fn default() -> Self {
        Self {
            active_slots: &[],
            include_frozen: true,
        }
    }
}

/// Result of building a slot filter SQL clause.
pub struct SlotFilter {
    /// The SQL WHERE clause fragment (including "WHERE").
    pub sql: String,
    /// Parameter values for the SQL clause.
    pub params: Vec<String>,
}

/// Build a SQL WHERE clause that filters documents to active slots only.
///
/// Frozen slots are always included regardless of `active_slots`.
/// Returns a `SlotFilter` with the SQL fragment and parameter values.
///
/// # Arguments
/// * `config` - Slot search configuration
///
/// # Returns
/// A `SlotFilter` containing the SQL WHERE clause and bound parameters.
/// If `active_slots` is empty and `include_frozen` is false, returns a clause
/// that matches nothing. If `include_frozen` is true and no active slots are
/// specified, only frozen slot documents are returned.
pub fn build_slot_filter(config: &SlotSearchConfig) -> SlotFilter {
    let mut params = Vec::new();
    let mut conditions = Vec::new();

    // Always include frozen slots if requested
    if config.include_frozen {
        conditions.push("s.is_frozen = TRUE".to_string());
    }

    // Add active slot conditions
    if !config.active_slots.is_empty() {
        let placeholders: Vec<String> = config
            .active_slots
            .iter()
            .enumerate()
            .map(|(i, name)| {
                params.push(name.to_string());
                format!("?{}", i + 1)
            })
            .collect();

        let in_clause = placeholders.join(", ");
        conditions.push(format!("sd.slot_name IN ({in_clause})"));
    }

    if conditions.is_empty() {
        // No active slots and no frozen — match nothing
        return SlotFilter {
            sql: "AND 1 = 0".to_string(),
            params,
        };
    }

    let where_clause = conditions.join(" OR ");
    let sql = format!(
        "AND d.id IN (\
            SELECT sd.document_id \
            FROM slot_documents sd \
            JOIN rag_slots s ON s.name = sd.slot_name \
            WHERE {where_clause}\
        )"
    );

    SlotFilter { sql, params }
}

/// Build the full slot-filtered document query SQL.
///
/// Returns a SQL query that selects documents with their decayed relevance scores,
/// filtered by the specified slot configuration.
pub fn slot_filtered_document_sql(config: &SlotSearchConfig) -> (String, Vec<String>) {
    let filter = build_slot_filter(config);

    let sql = format!(
        "SELECT\
            d.id,\
            d.title,\
            d.content,\
            d.source_url,\
            d.created_at,\
            COALESCE(\
                (\
                    SELECT MAX(\
                        CASE\
                            WHEN s.is_frozen = TRUE THEN sd.relevance_score\
                            ELSE sd.relevance_score * EXP(-s.decay_rate * (JULIANDAY('now') - JULIANDAY(sd.routed_at)))\
                        END\
                    )\
                    FROM slot_documents sd\
                    JOIN rag_slots s ON s.name = sd.slot_name\
                    WHERE sd.document_id = d.id\
                ),\
                0.0\
            ) AS slot_score\
        FROM documents d\
        WHERE 1 = 1\
        {filter_sql}\
        ORDER BY slot_score DESC",
        filter_sql = filter.sql
    );

    (sql, filter.params)
}

/// Build SQL to count documents per slot.
pub fn slot_document_counts_sql() -> &'static str {
    r#"
        SELECT
            s.name AS slot_name,
            COUNT(sd.document_id) AS document_count
        FROM rag_slots s
        LEFT JOIN slot_documents sd ON sd.slot_name = s.name
        GROUP BY s.name
        ORDER BY s.name
    "#
}

/// Build SQL to list documents in a specific slot with decayed scores.
pub fn slot_documents_sql(slot_name: &str) -> (String, Vec<String>) {
    let sql = r#"
        SELECT
            d.id,
            d.title,
            d.content,
            d.source_url,
            d.created_at,
            CASE
                WHEN s.is_frozen = TRUE THEN sd.relevance_score
                ELSE sd.relevance_score * EXP(-s.decay_rate * (JULIANDAY('now') - JULIANDAY(sd.routed_at)))
            END AS decayed_score,
            sd.routed_at,
            sd.routed_by
        FROM documents d
        JOIN slot_documents sd ON sd.document_id = d.id
        JOIN rag_slots s ON s.name = sd.slot_name
        WHERE sd.slot_name = ?1
        ORDER BY decayed_score DESC
    "#
    .to_string();

    (sql, vec![slot_name.to_string()])
}

/// Build SQL to remove a document from a specific slot.
pub fn remove_document_from_slot_sql() -> &'static str {
    "DELETE FROM slot_documents WHERE slot_name = ?1 AND document_id = ?2"
}

/// Build SQL to check if a document exists in any slot.
pub fn document_in_any_slot_sql() -> &'static str {
    "SELECT COUNT(*) as cnt FROM slot_documents WHERE document_id = ?1"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_filter_with_active_slots() {
        let config = SlotSearchConfig {
            active_slots: &[SlotName::Apis, SlotName::Types],
            include_frozen: true,
        };
        let filter = build_slot_filter(&config);

        assert!(filter.sql.contains("is_frozen = TRUE"));
        assert!(filter.sql.contains("sd.slot_name IN"));
        assert!(filter.sql.contains("SELECT sd.document_id"));
        assert_eq!(filter.params.len(), 2);
        assert_eq!(filter.params[0], "apis");
        assert_eq!(filter.params[1], "types");
    }

    #[test]
    fn test_slot_filter_frozen_only() {
        let config = SlotSearchConfig {
            active_slots: &[],
            include_frozen: true,
        };
        let filter = build_slot_filter(&config);

        assert!(filter.sql.contains("is_frozen = TRUE"));
        assert!(!filter.sql.contains("sd.slot_name IN"));
        assert!(filter.params.is_empty());
    }

    #[test]
    fn test_slot_filter_nothing() {
        let config = SlotSearchConfig {
            active_slots: &[],
            include_frozen: false,
        };
        let filter = build_slot_filter(&config);

        assert!(filter.sql.contains("1 = 0"));
        assert!(filter.params.is_empty());
    }

    #[test]
    fn test_slot_filter_active_only_no_frozen() {
        let config = SlotSearchConfig {
            active_slots: &[SlotName::Tests],
            include_frozen: false,
        };
        let filter = build_slot_filter(&config);

        assert!(!filter.sql.contains("is_frozen"));
        assert!(filter.sql.contains("sd.slot_name IN"));
        assert_eq!(filter.params.len(), 1);
        assert_eq!(filter.params[0], "tests");
    }

    #[test]
    fn test_slot_filtered_document_sql_contains_decay() {
        let config = SlotSearchConfig {
            active_slots: &[SlotName::Apis],
            include_frozen: true,
        };
        let (sql, params) = slot_filtered_document_sql(&config);

        assert!(sql.contains("EXP"));
        assert!(sql.contains("JULIANDAY"));
        assert!(sql.contains("slot_score"));
        assert!(sql.contains("ORDER BY slot_score DESC"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_slot_documents_sql() {
        let (sql, params) = slot_documents_sql("architecture");
        assert!(sql.contains("decayed_score"));
        assert!(sql.contains("sd.slot_name = ?1"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], "architecture");
    }

    #[test]
    fn test_slot_document_counts_sql() {
        let sql = slot_document_counts_sql();
        assert!(sql.contains("COUNT"));
        assert!(sql.contains("GROUP BY"));
    }

    #[test]
    fn test_custom_slot_name_in_filter() {
        let custom = SlotName::Custom("my_custom_slot".to_string());
        let config = SlotSearchConfig {
            active_slots: &[custom],
            include_frozen: false,
        };
        let filter = build_slot_filter(&config);

        assert_eq!(filter.params.len(), 1);
        assert_eq!(filter.params[0], "my_custom_slot");
    }

    #[test]
    fn test_default_config_includes_frozen() {
        let config = SlotSearchConfig::default();
        assert!(config.include_frozen);
        assert!(config.active_slots.is_empty());
    }

    #[test]
    fn test_remove_document_sql() {
        let sql = remove_document_from_slot_sql();
        assert!(sql.contains("DELETE FROM slot_documents"));
        assert!(sql.contains("?1"));
        assert!(sql.contains("?2"));
    }

    #[test]
    fn test_document_in_any_slot_sql() {
        let sql = document_in_any_slot_sql();
        assert!(sql.contains("COUNT"));
        assert!(sql.contains("slot_documents"));
    }
}
