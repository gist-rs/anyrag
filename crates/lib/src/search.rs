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
        ai::{generate_embedding, AiProvider},
        db::storage::{KeywordSearch, MetadataSearch, VectorSearch},
    },
    rerank::reciprocal_rank_fusion,
    types::SearchResult,
    PromptError,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, warn};

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
    QueryAnalysis(PromptError),
    #[error("Embedding generation failed: {0}")]
    Embedding(PromptError),
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
    provider: Arc<P>,
    ai_provider: Arc<dyn AiProvider>,
    query_text: String,
    owner_id: Option<String>,
    limit: u32,
    prompts: HybridSearchPrompts<'_>,
    use_keyword_search: bool,
    use_vector_search: bool,
    embedding_api_url: &str,
    embedding_model: &str,
) -> Result<Vec<SearchResult>, SearchError>
where
    P: MetadataSearch + VectorSearch + KeywordSearch + Send + Sync + 'static,
{
    let analyzed_query = analyze_query(
        ai_provider.as_ref(),
        &query_text,
        prompts.analysis_system_prompt,
        prompts.analysis_user_prompt_template,
    )
    .await
    .map_err(SearchError::QueryAnalysis)?;

    let candidate_doc_ids = provider
        .metadata_search(
            &analyzed_query.entities,
            &analyzed_query.keyphrases,
            owner_id.as_deref(),
            limit * 5,
        )
        .await?;

    let provider_kw = Arc::clone(&provider);
    let query_text_kw = query_text.clone();
    let owner_id_kw = owner_id.clone();
    let keyword_handle = tokio::spawn(async move {
        if use_keyword_search {
            provider_kw
                .keyword_search(&query_text_kw, limit * 2, owner_id_kw.as_deref())
                .await
        } else {
            Ok(Vec::new())
        }
    });

    let provider_vec = Arc::clone(&provider);
    let owner_id_vec = owner_id;
    let embedding_api_url = embedding_api_url.to_string();
    let embedding_model = embedding_model.to_string();
    let vector_handle = tokio::spawn(async move {
        if use_vector_search {
            let query_vector =
                generate_embedding(&embedding_api_url, &embedding_model, &query_text)
                    .await
                    .map_err(SearchError::Embedding)?;
            provider_vec
                .vector_search(
                    query_vector,
                    limit * 2,
                    owner_id_vec.as_deref(),
                    Some(&candidate_doc_ids),
                )
                .await
        } else {
            Ok(Vec::new())
        }
    });

    let (keyword_results, vector_results) = tokio::join!(keyword_handle, vector_handle);

    let keyword_candidates = match keyword_results {
        Ok(Ok(res)) => res,
        _ => Vec::new(),
    };

    let vector_candidates = match vector_results {
        Ok(Ok(res)) => res,
        _ => Vec::new(),
    };

    let mut final_results = reciprocal_rank_fusion(vector_candidates, keyword_candidates);
    final_results.truncate(limit as usize);
    Ok(final_results)
}
