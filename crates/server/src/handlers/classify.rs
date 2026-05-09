//! # Domain Classification Handler
//!
//! POST /classify/domain — Embedding-based domain classification for prompt routing.
//! Combines keyword overlap (30%) with vector embedding similarity (70%).
//! Falls back to keyword-only when the AI provider is unavailable.

use super::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use crate::auth::middleware::AuthenticatedUser;
use anyrag::{
    providers::{ai::generate_embeddings_batch, db::storage::VectorSearch},
    router::{ClassificationResult, DomainDefinition, HybridClassifier, ScoredDomain},
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
    pub candidate_domains: Vec<DomainDefinition>,
}

/// POST /classify/domain — Classify a prompt into a domain.
///
/// Combines keyword overlap (30%) with embedding similarity (70%).
/// Falls back to keyword-only if the AI provider is unavailable.
pub async fn classify_domain_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<ClassifyDomainRequest>,
) -> Result<Json<ApiResponse<ClassificationResult>>, AppError> {
    info!("Classifying domain for prompt: '{}'", payload.prompt);

    if payload.candidate_domains.is_empty() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "No candidate domains provided"
        )));
    }

    // Step 1: Compute keyword scores for all domains (pure, no I/O)
    let mut scored_domains: Vec<ScoredDomain> = payload
        .candidate_domains
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
    match compute_embedding_scores(
        &app_state,
        &payload.prompt,
        &payload.candidate_domains,
        &user.0.id,
    )
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
        "candidate_count": payload.candidate_domains.len(),
    });

    Ok(wrap_response(result, debug_params, Some(debug_info)))
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
