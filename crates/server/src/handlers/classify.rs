//! # Domain Classification Handler
//!
//! POST /classify/domain — Embedding-based domain classification for prompt routing.
//! Combines keyword overlap (30%) with vector embedding similarity (70%).
//! Falls back to keyword-only when the AI provider is unavailable.
//!
//! ## microgpt-rs Integration (Plan 005, Task 7)
//!
//! This endpoint is designed to be called by `microgpt-rs` as the V2 embedding-based
//! domain router, upgrading from the V1 `KeywordRouter` (Plan 023).
//!
//! ### How to configure `domains.toml`
//!
//! `microgpt-rs/domains.toml` domain names and keywords should match the
//! `[[domain_mapping]]` entries in anyrag's `config.yml`. The default anyrag
//! config already includes mappings for: `sudoku`, `pathfinding`, `rust_code`,
//! `py2rs`, `general`. To add custom domains, add entries to `config.yml`:
//!
//! ```yaml
//! domain_mappings:
//!   - domain: "my_domain"
//!     slots: ["apis"]
//!     keywords: ["my_keyword", "custom"]
//! ```
//!
//! ### How to call `/classify/domain` from microgpt-rs
//!
//! Send a POST request with the prompt and candidate domains:
//!
//! ```json
//! POST http://localhost:9090/classify/domain
//! Authorization: Bearer <jwt>
//! {
//!   "prompt": "Rewrite this FastAPI endpoint to Axum",
//!   "candidate_domains": [
//!     { "name": "rust_code", "keywords": ["rust", "cargo", "axum"], "slots": [] },
//!     { "name": "py2rs", "keywords": ["python", "rewrite", "fastapi"], "slots": [] }
//!   ]
//! }
//! ```
//!
//! Or omit `candidate_domains` to use the server's configured defaults:
//!
//! ```json
//! { "prompt": "solve this sudoku puzzle" }
//! ```
//!
//! Response:
//!
//! ```json
//! {
//!   "result": {
//!     "domain": "py2rs",
//!     "confidence": 0.85,
//!     "matched_slots": ["apis", "types"],
//!     "alternatives": [{ "domain": "rust_code", "confidence": 0.45 }]
//!   }
//! }
//! ```
//!
//! ### Fallback behavior when anyrag is unavailable
//!
//! If anyrag is down or unreachable, microgpt-rs should fall back to its
//! built-in `KeywordRouter` (Plan 023). The V1 keyword router is ~80% accurate
//! and requires no external service. Set a short timeout (200ms) on the REST
//! call to avoid blocking the prompt pipeline.

use super::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use crate::auth::middleware::AuthenticatedUser;
use anyrag::{
    providers::{ai::generate_embeddings_batch, db::storage::VectorSearch},
    router::{ClassificationResult, DomainDefinition, HybridClassifier, ScoredDomain},
    types::DomainMapping,
};
use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use tracing::{info, warn};

/// Request body for domain classification.
#[derive(Debug, Deserialize)]
pub struct ClassifyDomainRequest {
    /// The prompt to classify.
    pub prompt: String,
    /// Candidate domain definitions with keywords and slot associations.
    /// If empty, falls back to server-configured `domain_mappings` from config.
    #[serde(default)]
    pub candidate_domains: Vec<DomainDefinition>,
}

/// POST /classify/domain — Classify a prompt into a domain.
///
/// Combines keyword overlap (30%) with embedding similarity (70%).
/// Falls back to keyword-only if the AI provider is unavailable.
/// Falls back to config `domain_mappings` if no candidates provided.
pub async fn classify_domain_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<ClassifyDomainRequest>,
) -> Result<Json<ApiResponse<ClassificationResult>>, AppError> {
    info!("Classifying domain for prompt: '{}'", payload.prompt);

    // Resolve candidate domains: use request-provided, or fall back to config defaults.
    let candidate_domains = resolve_candidate_domains(&payload, &app_state.config.domain_mappings);

    if candidate_domains.is_empty() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "No candidate domains provided and no domain_mappings configured"
        )));
    }

    let used_defaults = payload.candidate_domains.is_empty();

    // Step 1: Compute keyword scores for all domains (pure, no I/O)
    let mut scored_domains: Vec<ScoredDomain> = candidate_domains
        .iter()
        .map(|domain| {
            let keyword_score = HybridClassifier::keyword_score(&payload.prompt, domain);
            ScoredDomain {
                domain: domain.name.clone(),
                keyword_score,
                embedding_score: None,
                matched_slots: vec![],
            }
        })
        .collect();

    // Step 2: Try embedding scoring (may fail — falls back to keyword-only)
    match compute_embedding_scores(&app_state, &payload.prompt, &candidate_domains, &user.0.id)
        .await
    {
        Ok(emb_scores) => {
            for sd in &mut scored_domains {
                if let Some((score, slots)) = emb_scores.get(&sd.domain) {
                    sd.embedding_score = Some(*score);
                    sd.matched_slots = slots.clone();
                }
            }
        }
        Err(e) => {
            warn!("Embedding scoring failed, using keyword-only: {e}");
        }
    }

    // Step 3: Classify using hybrid scoring
    let classifier = HybridClassifier::new();
    let result = classifier
        .classify_from_scores(scored_domains)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{e}")))?;

    info!(
        "Classified as '{}' (confidence: {:.2})",
        result.domain, result.confidence
    );

    let debug_info = json!({
        "prompt": payload.prompt,
        "candidate_count": candidate_domains.len(),
        "used_defaults": used_defaults,
    });

    Ok(wrap_response(result, debug_params, Some(debug_info)))
}

/// Resolve candidate domains from the request, or fall back to config defaults.
///
/// If the request provides `candidate_domains`, use those directly.
/// Otherwise, convert the server's `domain_mappings` config into `DomainDefinition`s.
fn resolve_candidate_domains(
    payload: &ClassifyDomainRequest,
    config_mappings: &[DomainMapping],
) -> Vec<DomainDefinition> {
    if !payload.candidate_domains.is_empty() {
        return payload.candidate_domains.clone();
    }

    config_mappings
        .iter()
        .map(|mapping| DomainDefinition {
            name: mapping.domain.clone(),
            keywords: mapping.keywords.clone(),
            slots: mapping.slots.clone(),
        })
        .collect()
}

/// Compute embedding-based scores for each domain.
///
/// For each domain that has slots configured:
/// 1. Query `slot_documents` to get document IDs in those slots
/// 2. Run vector search filtered to those documents
/// 3. Take the top similarity score as the domain's embedding score
///
/// Domains without slots get no embedding score (falls back to keyword-only).
async fn compute_embedding_scores(
    app_state: &AppState,
    prompt: &str,
    domains: &[DomainDefinition],
    owner_id: &str,
) -> Result<HashMap<String, (f32, Vec<String>)>, anyhow::Error> {
    let has_slots = domains.iter().any(|d| !d.slots.is_empty());
    if !has_slots {
        return Ok(HashMap::new());
    }

    // Embed the prompt (one API call, shared across all domains)
    let api_url = &app_state.config.embedding.api_url;
    let model = &app_state.config.embedding.model_name;
    let api_key = app_state.config.embedding.api_key.as_deref();

    let query_vector = generate_embeddings_batch(api_url, model, &[prompt], api_key)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No embedding returned for prompt"))?;

    let conn = app_state.sqlite_provider.db.connect()?;
    let mut domain_scores = HashMap::new();

    for domain in domains {
        if domain.slots.is_empty() {
            continue;
        }

        // Get document IDs for this domain's slots
        let placeholders: Vec<String> = domain
            .slots
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();
        let sql = format!(
            "SELECT DISTINCT document_id FROM slot_documents WHERE slot_name IN ({})",
            placeholders.join(", ")
        );
        let params: Vec<turso::Value> = domain.slots.iter().map(|s| s.clone().into()).collect();

        let mut rows = conn.query(&sql, params).await?;
        let mut doc_ids = Vec::new();
        while let Some(row) = rows.next().await? {
            if let Ok(id) = row.get::<String>(0) {
                doc_ids.push(id);
            }
        }
        drop(rows);

        if doc_ids.is_empty() {
            continue;
        }

        // Vector search filtered by these document IDs
        let results = app_state
            .sqlite_provider
            .vector_search(query_vector.clone(), 5, Some(owner_id), Some(&doc_ids))
            .await?;

        if let Some(top_result) = results.first() {
            let score = top_result.score.clamp(0.0, 1.0) as f32;
            // Report all configured slots as matched for this domain
            let matched_slots: Vec<String> = domain.slots.clone();
            domain_scores.insert(domain.name.clone(), (score, matched_slots));
        }
    }

    Ok(domain_scores)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_mappings() -> Vec<DomainMapping> {
        vec![
            DomainMapping {
                domain: "sudoku".to_string(),
                slots: vec!["tests".to_string()],
                keywords: vec![
                    "sudoku".to_string(),
                    "puzzle".to_string(),
                    "grid".to_string(),
                ],
            },
            DomainMapping {
                domain: "rust_code".to_string(),
                slots: vec!["apis".to_string(), "types".to_string()],
                keywords: vec!["rust".to_string(), "cargo".to_string(), "axum".to_string()],
            },
        ]
    }

    #[test]
    fn test_resolve_uses_request_candidates_when_provided() {
        let payload = ClassifyDomainRequest {
            prompt: "test".to_string(),
            candidate_domains: vec![DomainDefinition {
                name: "custom".to_string(),
                keywords: vec!["custom".to_string()],
                slots: vec![],
            }],
        };
        let result = resolve_candidate_domains(&payload, &sample_mappings());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "custom");
    }

    #[test]
    fn test_resolve_falls_back_to_config_defaults() {
        let payload = ClassifyDomainRequest {
            prompt: "test".to_string(),
            candidate_domains: vec![],
        };
        let result = resolve_candidate_domains(&payload, &sample_mappings());
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "sudoku");
        assert_eq!(result[1].name, "rust_code");
    }

    #[test]
    fn test_resolve_empty_when_no_config() {
        let payload = ClassifyDomainRequest {
            prompt: "test".to_string(),
            candidate_domains: vec![],
        };
        let result = resolve_candidate_domains(&payload, &[]);
        assert!(result.is_empty());
    }
}
