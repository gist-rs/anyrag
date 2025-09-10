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

/// Encapsulates all options for a hybrid search operation.
pub struct HybridSearchOptions<'a> {
    pub query_text: String,
    pub owner_id: Option<String>,
    pub limit: u32,
    pub prompts: HybridSearchPrompts<'a>,
    pub use_keyword_search: bool,
    pub use_vector_search: bool,
    pub embedding_api_url: &'a str,
    pub embedding_model: &'a str,
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
    #[error("A search task failed or panicked.")]
    TaskFailed,
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
    options: HybridSearchOptions<'_>,
) -> Result<Vec<SearchResult>, SearchError>
where
    P: MetadataSearch + VectorSearch + KeywordSearch + Send + Sync + 'static,
{
    let analyzed_query = analyze_query(
        ai_provider.as_ref(),
        &options.query_text,
        options.prompts.analysis_system_prompt,
        options.prompts.analysis_user_prompt_template,
    )
    .await
    .map_err(SearchError::QueryAnalysis)?;

    let candidate_doc_ids = provider
        .metadata_search(
            &analyzed_query.entities,
            &analyzed_query.keyphrases,
            options.owner_id.as_deref(),
            options.limit * 5,
        )
        .await?;

    let provider_kw = Arc::clone(&provider);
    let query_text_kw = options.query_text.clone();
    let owner_id_kw = options.owner_id.clone();
    let limit_kw = options.limit;
    let keyword_handle = tokio::spawn(async move {
        if options.use_keyword_search {
            provider_kw
                .keyword_search(&query_text_kw, limit_kw * 2, owner_id_kw.as_deref())
                .await
        } else {
            Ok(Vec::new())
        }
    });

    let provider_vec = Arc::clone(&provider);
    let owner_id_vec = options.owner_id;
    let embedding_api_url = options.embedding_api_url.to_string();
    let embedding_model = options.embedding_model.to_string();
    let query_text_vec = options.query_text;
    let limit_vec = options.limit;
    let vector_handle = tokio::spawn(async move {
        if options.use_vector_search {
            let query_vector =
                generate_embedding(&embedding_api_url, &embedding_model, &query_text_vec)
                    .await
                    .map_err(SearchError::Embedding)?;
            provider_vec
                .vector_search(
                    query_vector,
                    limit_vec * 2,
                    owner_id_vec.as_deref(),
                    Some(&candidate_doc_ids),
                )
                .await
        } else {
            Ok(Vec::new())
        }
    });

    let (keyword_results, vector_results) = tokio::join!(keyword_handle, vector_handle);

    // Soft-fail: If a task panics or returns an error, log it and proceed with an empty result set.
    let keyword_candidates = match keyword_results {
        Ok(Ok(res)) => res,
        Ok(Err(e)) => {
            warn!("Keyword search task failed: {}", e);
            Vec::new()
        }
        Err(e) => {
            warn!("Keyword search task panicked: {}", e);
            Vec::new()
        }
    };

    let vector_candidates = match vector_results {
        Ok(Ok(res)) => res,
        Ok(Err(e)) => {
            warn!("Vector search task failed: {}", e);
            Vec::new()
        }
        Err(e) => {
            warn!("Vector search task panicked: {}", e);
            Vec::new()
        }
    };

    let mut final_results = reciprocal_rank_fusion(vector_candidates, keyword_candidates);
    final_results.truncate(options.limit as usize);
    Ok(final_results)
}
