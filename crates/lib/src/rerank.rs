//! # Rerank Logic
//!
//! This module provides the core logic for all types of rerank:
//! - LLM.
//! - Reciprocal Rank Fusion.

use crate::{
    providers::ai::AiProvider,
    search::{SearchError, SearchResult},
};

use std::collections::HashMap;

use tracing::{debug, info};

/// Re-ranks search results using an LLM.
pub async fn llm_rerank(
    ai_provider: &dyn AiProvider,
    query_text: &str,
    candidates: Vec<SearchResult>,
) -> Result<Vec<SearchResult>, SearchError> {
    info!("Re-ranking {} candidates using LLM.", candidates.len());

    let system_prompt = "You are an expert search result re-ranker. Your task is to re-order a list of provided articles based on their relevance to a user's query. Analyze the user's query and the article content (title and description). Return a JSON array containing only the `link` strings of the articles in the new, correctly ordered sequence, from most relevant to least relevant. Do not add any explanation or other text outside of the JSON array.";

    let articles_context = candidates
        .iter()
        .enumerate()
        .map(|(i, r)| {
            format!(
                "Article {i}:\n- Title: {}\n- Link: {}\n- Description: {}",
                r.title, r.link, r.description
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let user_prompt = format!(
        "# User Query:\n{query_text}\n\n# Articles to Re-rank:\n{articles_context}\n\n# Your Output (JSON array of links only):\n"
    );

    debug!(system_prompt = %system_prompt, user_prompt = %user_prompt, "--> Sending prompt to LLM for re-ranking");

    let llm_response = ai_provider.generate(system_prompt, &user_prompt).await?;

    debug!("<-- LLM re-rank response: {}", llm_response);

    let re = regex::Regex::new(r"\[[\s\S]*\]").unwrap();
    let json_match = re.find(&llm_response).map(|m| m.as_str());

    let ordered_links: Vec<String> = match json_match {
        Some(json_str) => serde_json::from_str(json_str)?,
        None => {
            info!("LLM response did not contain a valid JSON array. Returning empty results.");
            return Ok(vec![]);
        }
    };

    let candidates_map: HashMap<String, SearchResult> = candidates
        .into_iter()
        .map(|c| (c.link.clone(), c))
        .collect();

    let final_results: Vec<SearchResult> = ordered_links
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
