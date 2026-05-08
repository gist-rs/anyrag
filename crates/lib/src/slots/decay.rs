//! # Selective Decay (Raven Equation 18)
//!
//! Implements per-slot decay following the Raven paper:
//!
//! ```text
//! score(t) = score_0 * exp(-λ * Δt)
//! ```
//!
//! Where:
//! - `score_0` = initial relevance score (1.0 at routing time)
//! - `λ` (lambda) = decay rate per slot (0.0 for frozen slots)
//! - `Δt` = time elapsed since routing in days
//!
//! Frozen slots (`is_frozen = TRUE`, `decay_rate = 0.0`) always return `exp(0) = 1.0`.

use chrono::{DateTime, Utc};

/// Minimum relevance score threshold. Documents below this are effectively decayed out.
pub const MIN_RELEVANCE_SCORE: f64 = 0.001;

/// Calculate the decayed relevance score using Raven Equation 18.
///
/// # Arguments
/// * `relevance_score` - Initial score (typically 1.0)
/// * `decay_rate` - λ (lambda), 0.0 for frozen slots
/// * `routed_at` - ISO 8601 timestamp when the document was routed
///
/// # Returns
/// The decayed score. For frozen slots (decay_rate = 0.0), returns the original score.
pub fn decayed_score(relevance_score: f64, decay_rate: f64, routed_at: &str) -> f64 {
    if decay_rate <= 0.0 {
        return relevance_score;
    }

    let delta_days = match elapsed_days(routed_at) {
        Ok(days) => days,
        Err(_) => return relevance_score, // On parse error, preserve original score
    };

    if delta_days <= 0.0 {
        return relevance_score;
    }

    let decayed = relevance_score * (-decay_rate * delta_days).exp();
    decayed.max(0.0)
}

/// Calculate elapsed days since a given ISO 8601 timestamp.
pub fn elapsed_days(routed_at: &str) -> Result<f64, chrono::ParseError> {
    let routed_time: DateTime<Utc> = DateTime::parse_from_rfc3339(routed_at)?.to_utc();
    let now = Utc::now();
    let delta = now - routed_time;
    Ok(delta.num_seconds() as f64 / 86400.0)
}

/// Calculate what the decayed score would be after a specific number of days.
/// Useful for testing and predictions without needing actual timestamps.
pub fn decayed_score_after_days(relevance_score: f64, decay_rate: f64, days: f64) -> f64 {
    if decay_rate <= 0.0 || days <= 0.0 {
        return relevance_score;
    }
    (relevance_score * (-decay_rate * days).exp()).max(0.0)
}

/// Check if a score has decayed below the minimum relevance threshold.
pub fn is_decayed_out(score: f64) -> bool {
    score < MIN_RELEVANCE_SCORE
}

/// Generate SQL to compute decayed scores for all non-frozen slot documents.
///
/// This SQL can be used in SELECT queries to get real-time decayed scores
/// without modifying stored data.
pub fn decay_select_sql() -> &'static str {
    r#"
        SELECT
            sd.id,
            sd.slot_name,
            sd.document_id,
            sd.routed_by,
            sd.routed_at,
            sd.relevance_score,
            CASE
                WHEN s.is_frozen = TRUE THEN sd.relevance_score
                ELSE sd.relevance_score * EXP(-s.decay_rate * (JULIANDAY('now') - JULIANDAY(sd.routed_at)))
            END AS decayed_score
        FROM slot_documents sd
        JOIN rag_slots s ON s.name = sd.slot_name
    "#
}

/// Generate SQL to batch-update relevance scores for all non-frozen slot documents.
/// This materializes the decay into the stored `relevance_score` column.
///
/// Should be called periodically (e.g., daily) to keep scores current.
pub fn apply_decay_batch_sql() -> &'static str {
    r#"
        UPDATE slot_documents
        SET relevance_score = (
            SELECT
                CASE
                    WHEN s.is_frozen = TRUE THEN slot_documents.relevance_score
                    ELSE slot_documents.relevance_score * EXP(-s.decay_rate * (JULIANDAY('now') - JULIANDAY(slot_documents.routed_at)))
                END
            FROM rag_slots s
            WHERE s.name = slot_documents.slot_name
        )
        WHERE slot_name IN (
            SELECT name FROM rag_slots WHERE is_frozen = FALSE
        )
    "#
}

/// Generate SQL to delete documents that have decayed below the minimum threshold.
pub fn cleanup_decayed_sql() -> String {
    format!(
        r#"
        DELETE FROM slot_documents
        WHERE slot_name IN (
            SELECT name FROM rag_slots WHERE is_frozen = FALSE
        )
        AND relevance_score < {MIN_RELEVANCE_SCORE}
    "#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frozen_slot_score_remains_unchanged() {
        let score = decayed_score_after_days(1.0, 0.0, 30.0);
        assert!(
            (score - 1.0).abs() < f64::EPSILON,
            "frozen slot should stay at 1.0, got {score}"
        );
    }

    #[test]
    fn test_chatter_decays_fast_after_7_days() {
        // λ=0.5, 7 days: exp(-0.5 * 7) = exp(-3.5) ≈ 0.0302
        let score = decayed_score_after_days(1.0, 0.5, 7.0);
        assert!(
            (score - 0.0302).abs() < 0.001,
            "chatter should decay to ~0.03 after 7 days, got {score}"
        );
        assert!(score > 0.0, "score should be positive");
    }

    #[test]
    fn test_types_decays_slowly_after_7_days() {
        // λ=0.05, 7 days: exp(-0.05 * 7) = exp(-0.35) ≈ 0.7047
        let score = decayed_score_after_days(1.0, 0.05, 7.0);
        assert!(
            (score - 0.70).abs() < 0.01,
            "types should decay to ~0.70 after 7 days, got {score}"
        );
    }

    #[test]
    fn test_dependencies_decays_after_30_days() {
        // λ=0.1, 30 days: exp(-0.1 * 30) = exp(-3.0) ≈ 0.0498
        let score = decayed_score_after_days(1.0, 0.1, 30.0);
        assert!(
            (score - 0.05).abs() < 0.01,
            "dependencies should decay to ~0.05 after 30 days, got {score}"
        );
    }

    #[test]
    fn test_zero_days_no_decay() {
        let score = decayed_score_after_days(1.0, 0.5, 0.0);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_negative_days_no_decay() {
        let score = decayed_score_after_days(1.0, 0.5, -1.0);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_negative_decay_rate_no_decay() {
        let score = decayed_score_after_days(1.0, -0.5, 10.0);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_initial_relevance_preserved_with_zero_decay() {
        let score = decayed_score_after_days(0.8, 0.0, 100.0);
        assert!((score - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_non_unit_initial_score() {
        // Initial score 0.5, λ=0.1, 7 days
        let score = decayed_score_after_days(0.5, 0.1, 7.0);
        let expected = 0.5 * (-0.1_f64 * 7.0).exp();
        assert!((score - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn test_is_decayed_out() {
        assert!(is_decayed_out(0.0001));
        assert!(is_decayed_out(0.0));
        assert!(!is_decayed_out(0.001));
        assert!(!is_decayed_out(0.5));
        assert!(!is_decayed_out(1.0));
    }

    #[test]
    fn test_chatter_nearly_zero_after_14_days() {
        // λ=0.5, 14 days: exp(-7.0) ≈ 0.0009
        let score = decayed_score_after_days(1.0, 0.5, 14.0);
        assert!(
            score < 0.001,
            "chatter should be nearly zero after 14 days, got {score}"
        );
        assert!(is_decayed_out(score));
    }

    #[test]
    fn test_decayed_score_with_recent_timestamp() {
        let now = Utc::now().to_rfc3339();
        let score = decayed_score(1.0, 0.5, &now);
        // Just routed, so score should be very close to 1.0
        assert!(
            (score - 1.0).abs() < 0.01,
            "recently routed doc should have score near 1.0, got {score}"
        );
    }

    #[test]
    fn test_decayed_score_with_invalid_timestamp() {
        let score = decayed_score(1.0, 0.5, "not-a-timestamp");
        // On parse error, preserve original score
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_elapsed_days_with_valid_timestamp() {
        let one_hour_ago = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        let days = elapsed_days(&one_hour_ago).unwrap();
        assert!(
            (days - (1.0 / 24.0)).abs() < 0.01,
            "1 hour ago should be ~0.042 days, got {days}"
        );
    }

    #[test]
    fn test_decay_select_sql_contains_frozen_check() {
        let sql = decay_select_sql();
        assert!(sql.contains("is_frozen"));
        assert!(sql.contains("EXP"));
        assert!(sql.contains("JULIANDAY"));
    }

    #[test]
    fn test_apply_decay_batch_sql_only_updates_non_frozen() {
        let sql = apply_decay_batch_sql();
        assert!(sql.contains("is_frozen = FALSE"));
        assert!(sql.contains("UPDATE slot_documents"));
    }

    #[test]
    fn test_cleanup_decayed_sql_uses_threshold() {
        let sql = cleanup_decayed_sql();
        assert!(sql.contains(&format!("{MIN_RELEVANCE_SCORE}")));
        assert!(sql.contains("DELETE FROM slot_documents"));
    }

    #[test]
    fn test_long_term_decay_architecture_stays_full() {
        // Architecture: frozen, 365 days — should still be 1.0
        let score = decayed_score_after_days(1.0, 0.0, 365.0);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_long_term_decay_types_still_relevant() {
        // Types: λ=0.05, 30 days: exp(-1.5) ≈ 0.223
        let score = decayed_score_after_days(1.0, 0.05, 30.0);
        assert!(
            score > 0.2,
            "types should still be relevant after 30 days, got {score}"
        );
    }
}
