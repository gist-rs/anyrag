//! # Rerank Logic
//!
//! This module provides the core logic for all types of rerank:
//! - LLM.
//! - Reciprocal Rank Fusion.

use crate::{
    providers::ai::AiProvider,
    types::{QueryContext, SearchResult, SearchSourceType},
    PromptError,
};
use std::{collections::HashMap, fmt::Debug};
use thiserror::Error;
use tracing::{debug, info};

/// Custom error types for the reranking process.
#[derive(Error, Debug)]
pub enum RerankError {
    #[error("LLM Re-ranking failed: {0}")]
    Llm(#[from] PromptError),
    #[error("Failed to parse LLM re-ranking response: {0}")]
    LlmResponseParsing(#[from] serde_json::Error),
}

/// A trait for items that can be re-ranked.
///
/// This allows the re-ranking logic to be generic over different types of
/// documents, as long as they can provide the necessary context for the LLM.
pub trait Rerankable: Clone + Debug {
    /// Returns a unique identifier for the item, such as a URL or a database ID.
    fn get_link(&self) -> &str;
    /// Returns the main title or heading of the item.
    fn get_title(&self) -> &str;
    /// Returns a summary or description of the item.
    fn get_description(&self) -> &str;
}

/// Configurable weights for Reciprocal Rank Fusion.
///
/// Controls how different search sources and content types are weighted
/// during fusion. This enables RIIR-aware search that boosts code results
/// and dampens documentation when generating code.
#[derive(Debug, Clone, Copy)]
pub struct RrfWeights {
    /// Weight for metadata search results (first result set). Default: 100.0.
    pub metadata: f64,
    /// Weight for vector search results (second result set). Default: 1.0.
    pub vector: f64,
    /// Weight for keyword search results (third result set). Default: 1.0.
    pub keyword: f64,
    /// Multiplier applied to results tagged as `SearchSourceType::Code`.
    /// Default: 10.0 — strongly boosts code results for RIIR tasks.
    pub code_boost: f64,
    /// Multiplier applied to results tagged as `SearchSourceType::Documentation`.
    /// Default: 0.5 — dampens prose for code generation queries.
    pub doc_penalty: f64,
}

impl Default for RrfWeights {
    fn default() -> Self {
        Self {
            metadata: 100.0,
            vector: 1.0,
            keyword: 1.0,
            code_boost: 1.0,
            doc_penalty: 1.0,
        }
    }
}

impl RrfWeights {
    /// Returns weights configured for the given query context.
    pub fn from_context(context: QueryContext) -> Self {
        match context {
            QueryContext::CodeGeneration => Self {
                code_boost: 10.0,
                doc_penalty: 0.5,
                ..Default::default()
            },
            QueryContext::Explanation => Self::default(),
            QueryContext::Debugging => Self {
                code_boost: 5.0,
                doc_penalty: 0.8,
                ..Default::default()
            },
        }
    }

    /// Returns the weight for a result set at the given index.
    ///
    /// Convention: 0 = metadata, 1 = vector, 2+ = keyword.
    pub fn set_weight(&self, set_index: usize) -> f64 {
        match set_index {
            0 => self.metadata,
            1 => self.vector,
            _ => self.keyword,
        }
    }

    /// Returns the source-type multiplier for a result.
    pub fn source_multiplier(&self, source_type: &SearchSourceType) -> f64 {
        match source_type {
            SearchSourceType::Code => self.code_boost,
            SearchSourceType::Documentation | SearchSourceType::Faq => self.doc_penalty,
            SearchSourceType::Unknown => 1.0,
        }
    }
}

/// Re-ranks a list of candidates using an LLM.
///
/// This function is generic and can re-rank any type that implements `Rerankable`.
pub async fn llm_rerank<T: Rerankable>(
    ai_provider: &dyn AiProvider,
    query_text: &str,
    candidates: Vec<T>,
    system_prompt: &str,
    user_prompt_template: &str,
) -> Result<Vec<T>, RerankError> {
    info!(
        "Re-ranking {} candidates using LLM for query: '{}'",
        candidates.len(),
        query_text
    );

    let articles_context = candidates
        .iter()
        .enumerate()
        .map(|(i, r)| {
            format!(
                "Article {i}:\n- Title: {}\n- Link: {}\n- Description: {}",
                r.get_title(),
                r.get_link(),
                r.get_description()
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let user_prompt = user_prompt_template
        .replace("{query_text}", query_text)
        .replace("{articles_context}", &articles_context);

    debug!(system_prompt = %system_prompt, user_prompt = %user_prompt, "--> Sending prompt to LLM for re-ranking");

    let llm_response = ai_provider.generate(system_prompt, &user_prompt).await?;

    debug!("<-- LLM re-rank response: {}", llm_response);

    // Extract the JSON array from the markdown code block for robustness.
    // Tries to find a ```json block first, then falls back to a raw array.
    let re = regex::Regex::new(r"```json\s*([\s\S]*?)\s*```|(\[[\s\S]*\])")
        .map_err(|e| RerankError::Llm(PromptError::Regex(e)))?;
    let json_match = re.find(&llm_response).map(|m| m.as_str());

    let ordered_links: Vec<String> = match json_match {
        Some(json_str) => {
            // The regex might capture the ```json ... ``` wrapper, so we clean it up.
            let cleaned_json = json_str
                .trim()
                .trim_start_matches("```json")
                .trim_end_matches("```")
                .trim();
            serde_json::from_str(cleaned_json)?
        }
        None => {
            info!("LLM response did not contain a valid JSON array. Returning empty results.");
            return Ok(vec![]);
        }
    };

    let candidates_map: HashMap<String, T> = candidates
        .into_iter()
        .map(|c| (c.get_link().to_string(), c))
        .collect();

    let final_results: Vec<T> = ordered_links
        .into_iter()
        .filter_map(|link| candidates_map.get(&link).cloned())
        .collect();

    Ok(final_results)
}

/// Re-ranks search results from multiple sources using Reciprocal Rank Fusion.
///
/// This is a convenience wrapper around [`reciprocal_rank_fusion_weighted`]
/// with default weights (no source-type boost/penalty).
pub fn reciprocal_rank_fusion(result_sets: Vec<Vec<SearchResult>>) -> Vec<SearchResult> {
    reciprocal_rank_fusion_weighted(&result_sets, &RrfWeights::default())
}

/// Re-ranks search results from multiple sources using weighted Reciprocal Rank Fusion.
///
/// Each result set is weighted according to its position (metadata/vector/keyword),
/// and individual results are boosted or penalized based on their [`SearchSourceType`].
///
/// # Arguments
///
/// * `result_sets` — Ordered slices: metadata (0), vector (1), keyword (2+).
/// * `weights` — Configurable weights for sources and content types.
pub fn reciprocal_rank_fusion_weighted(
    result_sets: &[Vec<SearchResult>],
    weights: &RrfWeights,
) -> Vec<SearchResult> {
    info!(
        "Re-ranking using Weighted Reciprocal Rank Fusion for {} result sets.",
        result_sets.len()
    );

    let mut rrf_scores: HashMap<String, f64> = HashMap::new();
    let k = 60.0; // Standard RRF constant

    let mut all_unique_results: HashMap<String, SearchResult> = HashMap::new();

    for (set_index, results) in result_sets.iter().enumerate() {
        let set_weight = weights.set_weight(set_index);
        for (rank, result) in results.iter().enumerate() {
            // A document is unique by its link *and* its content. This prevents
            // different versions of the same document (same link) from being de-duplicated.
            let unique_key = format!("{}::{}", result.link, result.description);

            // Base RRF score weighted by source set (metadata vs vector vs keyword).
            let base_score = set_weight / (k + (rank + 1) as f64);
            // Apply source-type boost/penalty (code_boost or doc_penalty).
            let source_multiplier = weights.source_multiplier(&result.source_type);
            let score = base_score * source_multiplier;
            debug!(
                "Weighted RRF score for '{}' (set: {}, rank: {}, source_type: {:?}): {} (base={}, mult={})",
                result.title, set_index, rank, result.source_type, score, base_score, source_multiplier
            );
            *rrf_scores.entry(unique_key.clone()).or_insert(0.0) += score;

            // Collect unique results by link+description key
            all_unique_results
                .entry(unique_key)
                .or_insert_with(|| result.clone());
        }
    }

    if all_unique_results.is_empty() {
        return Vec::new();
    }

    let mut combined_results: Vec<SearchResult> = all_unique_results.into_values().collect();

    combined_results.sort_by(|a, b| {
        let key_a = format!("{}::{}", a.link, a.description);
        let score_a = rrf_scores.get(&key_a).unwrap_or(&0.0);
        let key_b = format!("{}::{}", b.link, b.description);
        let score_b = rrf_scores.get(&key_b).unwrap_or(&0.0);
        score_b
            .partial_cmp(score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Update the final score in each result for debugging/transparency
    for result in &mut combined_results {
        let key = format!("{}::{}", result.link, result.description);
        result.score = *rrf_scores.get(&key).unwrap_or(&0.0);
    }

    debug!("Final weighted RRF scores: {:?}", rrf_scores);
    combined_results
}
