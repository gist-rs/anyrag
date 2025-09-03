//! # Search Logic
//!
//! This module provides the core logic for the multi-stage hybrid search pipeline.
//! The flow is designed to be both fast and relevant:
//! 1.  **Query Analysis**: An LLM extracts key entities and concepts from the user's query.
//! 2.  **Metadata Pre-Filtering**: A fast SQL query finds documents tagged with that metadata.
//! 3.  **Vector Re-Ranking**: A semantic vector search is performed *only* on the pre-filtered
//!     documents to find the final, most relevant results.

use crate::{
    providers::{
        ai::AiProvider,
        db::storage::{MetadataSearch, VectorSearch},
    },
    types::SearchResult,
    PromptError,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};

/// Defines the re-ranking strategy for hybrid search.
#[derive(Default, Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    /// Uses a Large Language Model to intelligently re-rank candidates. (Default)
    #[default]
    LlmReRank,
    /// Uses the fast Reciprocal Rank Fusion algorithm.
    Rrf,
}

/// A struct to hold the prompts for the hybrid search query analysis step.
pub struct HybridSearchPrompts<'a> {
    pub analysis_system_prompt: &'a str,
    pub analysis_user_prompt_template: &'a str,
}

// --- Query Analysis ---

#[derive(Deserialize, Debug)]
struct AnalyzedQuery {
    #[serde(default)]
    entities: Vec<String>,
    #[serde(default)]
    keyphrases: Vec<String>,
}

/// Custom error types for the search process.
#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Database error: {0}")]
    Database(#[from] turso::Error),
    #[error("Query analysis failed: {0}")]
    QueryAnalysis(#[from] PromptError),
}

/// Uses an LLM to extract entities and keyphrases from a user query.
async fn analyze_query(
    ai_provider: &dyn AiProvider,
    query_text: &str,
    system_prompt: &str,
    user_prompt_template: &str,
) -> Result<AnalyzedQuery, PromptError> {
    let user_prompt = user_prompt_template.replace("{prompt}", query_text);
    let llm_response = ai_provider.generate(system_prompt, &user_prompt).await?;

    debug!("LLM query analysis response: {}", llm_response);
    let cleaned_response = llm_response
        .trim()
        .strip_prefix("```json")
        .unwrap_or(&llm_response)
        .strip_suffix("```")
        .unwrap_or(&llm_response)
        .trim();

    match serde_json::from_str(cleaned_response) {
        Ok(parsed) => Ok(parsed),
        Err(e) => {
            warn!(
                "Failed to parse query analysis JSON, falling back to using full query as keyphrase. Error: {}. Raw response: '{}'",
                e, cleaned_response
            );
            // Fallback: use the original query as a keyphrase
            Ok(AnalyzedQuery {
                entities: Vec::new(),
                keyphrases: vec![query_text.to_string()],
            })
        }
    }
}

/// Performs a multi-stage hybrid search.
pub async fn hybrid_search<P>(
    provider: &P,
    ai_provider: &dyn AiProvider,
    query_vector: Vec<f32>,
    query_text: &str,
    owner_id: Option<&str>, // For security
    limit: u32,
    prompts: HybridSearchPrompts<'_>,
) -> Result<Vec<SearchResult>, SearchError>
where
    P: MetadataSearch + VectorSearch + ?Sized,
{
    info!("Starting multi-stage hybrid search for: '{}'", query_text);

    // --- Stage 1: Query Analysis ---
    let analyzed_query = analyze_query(
        ai_provider,
        query_text,
        prompts.analysis_system_prompt,
        prompts.analysis_user_prompt_template,
    )
    .await?;
    info!("Analyzed query: {:?}", analyzed_query);

    // --- Stage 2: Metadata Pre-Filtering ---
    // Fetch a larger pool of candidates from metadata to give the vector search more to work with.
    let candidate_doc_ids = provider
        .metadata_search(
            &analyzed_query.entities,
            &analyzed_query.keyphrases,
            owner_id,
            limit * 5, // Fetch 5x the final limit
        )
        .await?;

    info!(
        "Metadata search found {} candidate documents.",
        candidate_doc_ids.len()
    );

    if candidate_doc_ids.is_empty() {
        // Fallback: If no metadata matches, perform a broad vector search.
        info!("No candidates from metadata, falling back to broad vector search.");
        return provider
            .vector_search(query_vector, limit, owner_id, None)
            .await;
    }

    // --- Stage 3: Vector Re-Ranking ---
    // Perform a vector search restricted to the candidate document IDs.
    let ranked_results = provider
        .vector_search(query_vector, limit, owner_id, Some(&candidate_doc_ids))
        .await?;

    info!(
        "Vector search re-ranked to {} final results.",
        ranked_results.len()
    );
    Ok(ranked_results)
}
