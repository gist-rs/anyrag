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
// --- Query Analysis ---

const QUERY_ANALYSIS_SYSTEM_PROMPT: &str = r#"You are an expert query analyst. Your task is to extract key **Entities** and **Keyphrases** from the user's query to be used for a database search.

# Instructions:
1.  **Entities**: Identify specific, proper nouns (e.g., product names, people, organizations).
2.  **Keyphrases**: Identify the main concepts or topics.
3.  Return a single JSON object with two keys: `entities` and `keyphrases`.

# Example:
## USER QUERY:
What are the conditions for the True App Mega Campaign to win a Tesla?

## YOUR JSON OUTPUT:
{
  "entities": ["True App", "Tesla"],
  "keyphrases": ["campaign conditions", "win tesla"]
}"#;

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
) -> Result<AnalyzedQuery, PromptError> {
    let llm_response = ai_provider
        .generate(QUERY_ANALYSIS_SYSTEM_PROMPT, query_text)
        .await?;

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
                "Failed to parse query analysis, falling back to using full query as keyphrase. Error: {}",
                e
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
) -> Result<Vec<SearchResult>, SearchError>
where
    P: MetadataSearch + VectorSearch + ?Sized,
{
    info!("Starting multi-stage hybrid search for: '{}'", query_text);

    // --- Stage 1: Query Analysis ---
    let analyzed_query = analyze_query(ai_provider, query_text).await?;
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
        return provider.vector_search(query_vector, limit, None).await;
    }

    // --- Stage 3: Vector Re-Ranking ---
    // Perform a vector search restricted to the candidate document IDs.
    let ranked_results = provider
        .vector_search(query_vector, limit, Some(&candidate_doc_ids))
        .await?;

    info!(
        "Vector search re-ranked to {} final results.",
        ranked_results.len()
    );
    Ok(ranked_results)
}
