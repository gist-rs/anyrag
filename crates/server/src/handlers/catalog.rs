//! # Catalog-Driven Domain Shaping API
//!
//! Plan 007: Endpoints for domain expert metadata, tokenization,
//! and catalog-driven behavior shaping.
//!
//! These endpoints mirror NVIDIA Dynamo's catalog pattern where metadata
//! (truncation policy, reasoning settings, inference budget) shapes agent
//! behavior as much as the model itself.

use super::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use anyrag::{
    router::types::{DomainHints, InferenceBudget, ReasoningPolicy, TruncationPolicy},
    types::DomainMapping,
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

// ---------------------------------------------------------------------------
// Domain Model Endpoints
// ---------------------------------------------------------------------------

/// Domain expert metadata response — what Dynamo's GET /v1/models/{model_id} returns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainModelResponse {
    /// Domain identifier (e.g., "py2rs").
    pub id: String,
    /// Human-readable domain name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Keywords for keyword-based routing.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Truncation policy (if configured).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation: Option<TruncationPolicy>,
    /// Reasoning retention policy (if configured).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ReasoningPolicy>,
    /// Agent hints (if configured).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hints: Option<DomainHints>,
    /// Inference budget (if configured).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference: Option<InferenceBudget>,
    /// Slot names used for embedding similarity.
    #[serde(default)]
    pub slots: Vec<String>,
    /// Context window size (derived from truncation limit or default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u32>,
}

/// GET /v1/models/{domain} — Returns metadata for a domain expert.
///
/// This is analogous to NVIDIA Dynamo's GET /v1/models/{model_id}.
/// Returns the domain's keywords, truncation policy, reasoning settings,
/// inference budget, and agent hints.
pub async fn get_domain_model_handler(
    State(app_state): State<AppState>,
    Path(domain): Path<String>,
    debug_params: Query<DebugParams>,
) -> Result<Json<ApiResponse<DomainModelResponse>>, AppError> {
    let mapping = app_state
        .config
        .domain_mappings
        .iter()
        .find(|m| m.domain == domain)
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Domain '{domain}' not found in configuration"
            ))
        })?;

    let response = domain_mapping_to_response(mapping);

    let debug_info = json!({
        "domain": domain,
    });

    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

/// List all domain models.
/// GET /v1/models — Returns metadata for all configured domain experts.
pub async fn list_domain_models_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
) -> Result<Json<ApiResponse<Vec<DomainModelResponse>>>, AppError> {
    let models: Vec<DomainModelResponse> = app_state
        .config
        .domain_mappings
        .iter()
        .map(domain_mapping_to_response)
        .collect();

    let debug_info = json!({
        "count": models.len(),
    });

    Ok(wrap_response(models, debug_params, Some(debug_info)))
}

/// Convert a `DomainMapping` config entry into an API response.
fn domain_mapping_to_response(mapping: &DomainMapping) -> DomainModelResponse {
    let context_window = mapping.truncation.as_ref().map(|t| t.limit);
    let inference = mapping.inference.as_ref().map(|b| b.resolve());

    DomainModelResponse {
        id: mapping.domain.clone(),
        name: None, // Could be derived from domain name in future
        keywords: mapping.keywords.clone(),
        truncation: mapping.truncation.clone(),
        reasoning: mapping.reasoning.clone(),
        hints: mapping.hints.clone(),
        inference,
        slots: mapping.slots.clone(),
        context_window,
    }
}

// ---------------------------------------------------------------------------
// Tokenize / Detokenize Stubs
// ---------------------------------------------------------------------------

/// Request body for tokenization.
#[derive(Debug, Deserialize)]
pub struct TokenizeRequest {
    /// Text to tokenize.
    pub text: String,
}

/// Response for tokenization.
#[derive(Debug, Serialize)]
pub struct TokenizeResponse {
    /// Number of tokens in the text.
    pub token_count: usize,
    /// Token IDs (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<Vec<u32>>,
}

/// POST /v1/tokenize — Tokenize text and return token count.
///
/// Stub implementation: estimates token count as `text.len() / 4` (rough heuristic).
/// Full implementation requires integrating a tokenizer (e.g., tiktoken).
pub async fn tokenize_handler(
    Json(payload): Json<TokenizeRequest>,
) -> Result<Json<TokenizeResponse>, AppError> {
    // Rough heuristic: ~4 chars per token for English text
    let token_count = (payload.text.len() / 4).max(1);

    Ok(Json(TokenizeResponse {
        token_count,
        tokens: None,
    }))
}

/// Request body for detokenization.
#[derive(Debug, Deserialize)]
pub struct DetokenizeRequest {
    /// Token IDs to detokenize.
    pub tokens: Vec<u32>,
}

/// Response for detokenization.
#[derive(Debug, Serialize)]
pub struct DetokenizeResponse {
    /// Decoded text from tokens.
    pub text: String,
}

/// POST /v1/detokenize — Detokenize token IDs back to text.
///
/// Stub implementation: returns placeholder string.
/// Full implementation requires integrating a tokenizer (e.g., tiktoken).
pub async fn detokenize_handler(
    Json(_payload): Json<DetokenizeRequest>,
) -> Result<Json<DetokenizeResponse>, AppError> {
    // Stub: not yet implemented
    Err(AppError::Internal(anyhow::anyhow!(
        "Detokenization not yet implemented — requires tokenizer integration"
    )))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_mapping() -> DomainMapping {
        DomainMapping {
            domain: "py2rs".to_string(),
            slots: vec!["apis".to_string(), "types".to_string()],
            keywords: vec![
                "python".to_string(),
                "rewrite".to_string(),
                "fastapi".to_string(),
            ],
            truncation: Some(TruncationPolicy {
                mode: anyrag::router::types::TruncationMode::Tokens,
                limit: 10000,
            }),
            reasoning: Some(ReasoningPolicy {
                keep_on_tool_calls: true,
                keep_on_plain: false,
            }),
            hints: Some(DomainHints {
                latency_sensitivity: Some(0.8),
                speculative_prefill: true,
            }),
            inference: Some(InferenceBudget {
                tree_budget: Some(100),
                draft_lookahead: None,
                screening_threshold: None,
                temperature: None,
                beta: None,
            }),
        }
    }

    #[test]
    fn test_domain_mapping_to_response_basic_fields() {
        let mapping = sample_mapping();
        let response = domain_mapping_to_response(&mapping);

        assert_eq!(response.id, "py2rs");
        assert_eq!(response.name, None);
        assert_eq!(response.keywords, vec!["python", "rewrite", "fastapi"]);
        assert_eq!(response.slots, vec!["apis", "types"]);
    }

    #[test]
    fn test_domain_mapping_to_response_truncation_and_context() {
        let mapping = sample_mapping();
        let response = domain_mapping_to_response(&mapping);

        assert!(response.truncation.is_some());
        assert_eq!(response.context_window, Some(10000));
    }

    #[test]
    fn test_domain_mapping_to_response_reasoning() {
        let mapping = sample_mapping();
        let response = domain_mapping_to_response(&mapping);

        let reasoning = response.reasoning.expect("reasoning should be set");
        assert!(reasoning.keep_on_tool_calls);
        assert!(!reasoning.keep_on_plain);
    }

    #[test]
    fn test_domain_mapping_to_response_hints() {
        let mapping = sample_mapping();
        let response = domain_mapping_to_response(&mapping);

        let hints = response.hints.expect("hints should be set");
        assert_eq!(hints.latency_sensitivity, Some(0.8));
        assert!(hints.speculative_prefill);
    }

    #[test]
    fn test_domain_mapping_to_response_inference_resolved() {
        let mapping = sample_mapping();
        let response = domain_mapping_to_response(&mapping);

        let inference = response.inference.expect("inference should be set");
        assert_eq!(inference.tree_budget, Some(100));
    }

    #[test]
    fn test_domain_mapping_minimal() {
        let mapping = DomainMapping {
            domain: "minimal".to_string(),
            slots: vec![],
            keywords: vec![],
            truncation: None,
            reasoning: None,
            hints: None,
            inference: None,
        };
        let response = domain_mapping_to_response(&mapping);

        assert_eq!(response.id, "minimal");
        assert!(response.keywords.is_empty());
        assert!(response.slots.is_empty());
        assert!(response.truncation.is_none());
        assert!(response.reasoning.is_none());
        assert!(response.hints.is_none());
        assert!(response.inference.is_none());
        assert!(response.context_window.is_none());
    }

    #[test]
    fn test_domain_model_response_serde_roundtrip() {
        let mapping = sample_mapping();
        let response = domain_mapping_to_response(&mapping);

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: DomainModelResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response.id, deserialized.id);
        assert_eq!(response.keywords, deserialized.keywords);
        assert_eq!(response.slots, deserialized.slots);
    }
}
