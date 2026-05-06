//! Episodic memory for tracking RIIR translation success/failure.

use crate::types::{CompilationResult, EpisodicStats, TranslationEpisode};
use anyhow::Result;
use tracing::{info, warn};
use turso::{params, Connection, Value as TursoValue};

/// Insert a new translation episode into the database.
///
/// Records the full lifecycle of a RIIR translation: source code,
/// retrieved context, generated Rust, and (optionally) compilation result.
pub async fn record_episode(conn: &Connection, episode: &TranslationEpisode) -> Result<()> {
    let retrieved_context_json = serde_json::to_string(&episode.retrieved_context)?;
    let hidden_state_json = episode
        .hidden_state
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let compilation_result_json = serde_json::to_string(&episode.compilation_result)?;

    conn.execute(
        "INSERT INTO episodes (id, source_language, source_code, generated_rust, retrieved_context, hidden_state, compilation_result, created_at, synthesized) VALUES (?, ?, ?, ?, ?, ?, ?, ?, FALSE)",
        params![
            episode.id.clone(),
            episode.source_language.clone(),
            episode.source_code.clone(),
            episode.generated_rust.clone(),
            retrieved_context_json,
            hidden_state_json,
            compilation_result_json,
            episode.created_at.clone(),
        ],
    )
    .await?;

    info!(
        "Recorded episode: id={}, lang={}",
        episode.id, episode.source_language
    );
    Ok(())
}

/// Update compilation result for an existing episode.
///
/// Called after the generated Rust code has been compiled and verified.
pub async fn verify_episode(
    conn: &Connection,
    episode_id: &str,
    result: &CompilationResult,
) -> Result<()> {
    let compilation_result_json = serde_json::to_string(result)?;

    conn.execute(
        "UPDATE episodes SET compilation_result = ? WHERE id = ?",
        params![compilation_result_json, episode_id],
    )
    .await?;

    info!("Verified episode: id={}", episode_id);
    Ok(())
}

/// Get episodes with successful compilation for synthesis.
///
/// Returns episodes ordered by creation date (newest first).
/// Optionally filtered by a `since` ISO date string.
pub async fn get_successful_episodes(
    conn: &Connection,
    limit: usize,
    since: Option<&str>,
) -> Result<Vec<TranslationEpisode>> {
    let sql = match since {
        Some(_) => "SELECT id, source_language, source_code, generated_rust, retrieved_context, hidden_state, compilation_result, created_at FROM episodes WHERE json_extract(compilation_result, '$.type') = 'success' AND created_at > ? ORDER BY created_at DESC LIMIT ?",
        None => "SELECT id, source_language, source_code, generated_rust, retrieved_context, hidden_state, compilation_result, created_at FROM episodes WHERE json_extract(compilation_result, '$.type') = 'success' ORDER BY created_at DESC LIMIT ?",
    };

    let mut stmt = conn.prepare(sql).await?;

    let mut rows = match since {
        Some(since_date) => stmt.query(params![since_date, limit as i64]).await?,
        None => stmt.query(params![limit as i64]).await?,
    };

    let mut episodes = Vec::new();
    while let Some(row) = rows.next().await? {
        let id: String = row.get(0)?;
        let source_language: String = row.get(1)?;
        let source_code: String = row.get(2)?;
        let generated_rust: String = row.get(3)?;
        let retrieved_context_raw: String = row.get(4)?;
        let hidden_state_raw = row.get_value(5)?;
        let compilation_result_raw: String = row.get(6)?;
        let created_at: String = row.get(7)?;

        let retrieved_context = serde_json::from_str(&retrieved_context_raw)?;
        let hidden_state = match hidden_state_raw {
            TursoValue::Text(s) => Some(serde_json::from_str(&s)?),
            TursoValue::Null => None,
            other => {
                warn!("Unexpected hidden_state type for episode {id}: {other:?}");
                None
            }
        };
        let compilation_result = serde_json::from_str(&compilation_result_raw)?;

        episodes.push(TranslationEpisode {
            id,
            source_language,
            source_code,
            generated_rust,
            retrieved_context,
            hidden_state,
            compilation_result,
            created_at,
        });
    }

    info!("Retrieved {} successful episodes", episodes.len());
    Ok(episodes)
}

/// Get statistics about episodes.
///
/// Returns counts of total/successful/failed episodes, success rate,
/// and the top 5 most frequent error codes from failed compilations.
pub async fn get_stats(conn: &Connection) -> Result<EpisodicStats> {
    // Counts query
    let mut stmt = conn.prepare(
        "SELECT \
           COUNT(*) as total, \
           COALESCE(SUM(CASE WHEN json_extract(compilation_result, '$.type') = 'success' THEN 1 ELSE 0 END), 0) as successful, \
           COALESCE(SUM(CASE WHEN json_extract(compilation_result, '$.type') = 'failed' THEN 1 ELSE 0 END), 0) as failed \
         FROM episodes",
    ).await?;
    let mut rows = stmt.query(params![]).await?;

    let (total, successful, failed) = if let Some(row) = rows.next().await? {
        let total: i64 = row.get(0)?;
        let successful: i64 = row.get(1)?;
        let failed: i64 = row.get(2)?;
        (total as u64, successful as u64, failed as u64)
    } else {
        (0, 0, 0)
    };

    let success_rate = if total > 0 {
        successful as f64 / total as f64
    } else {
        0.0
    };

    // Top error codes query
    let mut stmt = conn
        .prepare(
            "SELECT json_extract(compilation_result, '$.error_code') as code, COUNT(*) as count \
         FROM episodes \
         WHERE json_extract(compilation_result, '$.type') = 'failed' \
           AND json_extract(compilation_result, '$.error_code') IS NOT NULL \
         GROUP BY code \
         ORDER BY count DESC \
         LIMIT 5",
        )
        .await?;
    let mut rows = stmt.query(params![]).await?;

    let mut top_error_codes = Vec::new();
    while let Some(row) = rows.next().await? {
        let code: String = row.get(0)?;
        let count: i64 = row.get(1)?;
        top_error_codes.push((code, count as u64));
    }

    Ok(EpisodicStats {
        total_episodes: total,
        successful,
        failed,
        success_rate,
        top_error_codes,
    })
}

/// Mark episodes as synthesized after they've been used for training data.
///
/// This prevents the same episodes from being re-used in future synthesis rounds.
pub async fn mark_synthesized(conn: &Connection, episode_ids: &[String]) -> Result<()> {
    if episode_ids.is_empty() {
        return Ok(());
    }

    let placeholders: Vec<&str> = episode_ids.iter().map(|_| "?").collect();
    let sql = format!(
        "UPDATE episodes SET synthesized = TRUE WHERE id IN ({})",
        placeholders.join(", ")
    );

    let param_values: Vec<TursoValue> = episode_ids
        .iter()
        .map(|id| TursoValue::Text(id.clone()))
        .collect();

    conn.execute(&sql, param_values).await?;

    info!("Marked {} episodes as synthesized", episode_ids.len());
    Ok(())
}
