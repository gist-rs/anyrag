//! # Rerank Logic
//!
//! This module provides the core logic for all types of rerank:
//! - LLM.
//! - Reciprocal Rank Fusion.

use crate::{
    prompts::{DEFAULT_RERANK_SYSTEM_PROMPT, DEFAULT_RERANK_USER_PROMPT},
    providers::ai::AiProvider,
    types::SearchResult,
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

/// Re-ranks a list of candidates using an LLM.
///
/// This function is generic and can re-rank any type that implements `Rerankable`.
pub async fn llm_rerank<T: Rerankable>(
    ai_provider: &dyn AiProvider,
    query_text: &str,
    candidates: Vec<T>,
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

    let user_prompt = DEFAULT_RERANK_USER_PROMPT
        .replace("{query_text}", query_text)
        .replace("{articles_context}", &articles_context);

    debug!(system_prompt = %DEFAULT_RERANK_SYSTEM_PROMPT, user_prompt = %user_prompt, "--> Sending prompt to LLM for re-ranking");

    let llm_response = ai_provider
        .generate(DEFAULT_RERANK_SYSTEM_PROMPT, &user_prompt)
        .await?;

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

/// Re-ranks search results using Reciprocal Rank Fusion.
pub fn reciprocal_rank_fusion(
    vector_results: Vec<SearchResult>,
    keyword_results: Vec<SearchResult>,
) -> Vec<SearchResult> {
    info!("Re-ranking using Reciprocal Rank Fusion.");

    let mut rrf_scores: HashMap<String, f64> = HashMap::new();
    let k = 60.0;
    let keyword_boost = 1.2;

    for (i, result) in vector_results.iter().enumerate() {
        let rank = (i + 1) as f64;
        *rrf_scores.entry(result.link.clone()).or_insert(0.0) += 1.0 / (k + rank);
    }

    for (i, result) in keyword_results.iter().enumerate() {
        let rank = (i + 1) as f64;
        let score = (1.0 / (k + rank)) * keyword_boost;
        *rrf_scores.entry(result.link.clone()).or_insert(0.0) += score;
    }

    let mut combined_results: Vec<SearchResult> = vector_results
        .into_iter()
        .chain(keyword_results)
        .map(|res| (res.link.clone(), res))
        .collect::<HashMap<_, _>>()
        .into_values()
        .collect();

    combined_results.sort_by(|a, b| {
        let score_a = rrf_scores.get(&a.link).unwrap_or(&0.0);
        let score_b = rrf_scores.get(&b.link).unwrap_or(&0.0);
        score_b
            .partial_cmp(score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    info!("RRF sorted results: {:?}", combined_results);

    for result in &mut combined_results {
        result.score = *rrf_scores.get(&result.link).unwrap_or(&0.0);
    }

    combined_results
}
