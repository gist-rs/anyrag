//! # Search Logic
//!
//! This module provides the core logic for the multi-stage hybrid search pipeline.
//! The flow is designed to be both fast and relevant:
//! 1.  **Query Analysis**: An LLM extracts key entities and concepts from the user's query.
//! 2.  **Parallel Retrieval**: Metadata, keyword, and vector searches are run concurrently to gather a wide set of candidate documents.
//! 3.  **Re-ranking**: The results from all sources are combined and re-ranked using Reciprocal Rank Fusion to produce the final, most relevant results.

use crate::ingest::knowledge::clean_llm_response;
use crate::{
    providers::{
        ai::{generate_embeddings_batch, AiProvider},
        db::storage::{KeywordSearch, MetadataSearch, TemporalSearch, VectorSearch},
    },
    rerank::reciprocal_rank_fusion,
    types::SearchResult,
    PromptError,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_yaml;

use std::sync::Arc;
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

/// Encapsulates all options for a hybrid search operation.
/// Configuration for temporal ranking.
#[derive(Clone, Copy)]
pub struct TemporalRankingConfig<'a> {
    /// Keywords that trigger temporal ranking (e.g., "newest", "latest").
    pub keywords: &'a [&'a str],
    /// The name of the metadata property that holds the date/timestamp.
    pub property_name: &'a str,
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
    pub embedding_api_key: Option<&'a str>,
    pub temporal_ranking_config: Option<TemporalRankingConfig<'a>>,
}

// --- Query Analysis ---

#[derive(Deserialize, Debug)]
struct AnalyzedQuery {
    #[serde(default)]
    entities: Vec<String>,
    #[serde(default)]
    keyphrases: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct Faq {
    question: String,
    answer: String,
}

#[derive(Deserialize, Debug, Clone)]
struct Section {
    title: String,
    faqs: Vec<Faq>,
}

#[derive(Deserialize, Debug, Clone)]
struct YamlContent {
    sections: Vec<Section>,
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
    let cleaned_response = clean_llm_response(&llm_response);

    match serde_json::from_str(&cleaned_response) {
        Ok(parsed) => Ok(parsed),
        Err(e) => {
            warn!(
                "Failed to parse query analysis JSON, falling back to using full query as keyphrase. Error: {}. Raw response: '{}'",
                e, &cleaned_response
            );
            // Fallback: use the original query as a keyphrase
            Ok(AnalyzedQuery {
                entities: Vec::new(),
                keyphrases: vec![query_text.to_string()],
            })
        }
    }
}

/// Sorts search results based on a date property if found.
async fn temporally_rank_results<P>(
    provider: Arc<P>,
    results: Vec<SearchResult>,
    config: &TemporalRankingConfig<'_>,
    owner_id: Option<&str>,
) -> Vec<SearchResult>
where
    P: TemporalSearch + Send + Sync + 'static,
{
    let doc_ids: Vec<&str> = results.iter().map(|r| r.link.as_str()).collect();

    if doc_ids.is_empty() {
        return results;
    }

    let properties = match provider
        .get_string_properties_for_documents(&doc_ids, config.property_name, owner_id)
        .await
    {
        Ok(props) => props,
        Err(e) => {
            warn!(
                "Failed to fetch temporal properties, returning original order: {}",
                e
            );
            return results;
        }
    };

    let mut dated_results: Vec<(SearchResult, NaiveDate)> = results
        .into_iter()
        .filter_map(|r| {
            properties.get(&r.link).and_then(|date_str| {
                NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                    .ok()
                    .map(|date| (r, date))
            })
        })
        .collect();

    // Sort by date descending (newest first)
    dated_results.sort_by(|a, b| b.1.cmp(&a.1));

    dated_results.into_iter().map(|(r, _)| r).collect()
}

/// Performs a multi-stage hybrid search.
pub async fn hybrid_search<P>(
    provider: Arc<P>,
    ai_provider: Arc<dyn AiProvider>,
    options: HybridSearchOptions<'_>,
) -> Result<Vec<SearchResult>, SearchError>
where
    P: MetadataSearch + VectorSearch + KeywordSearch + TemporalSearch + Send + Sync + 'static,
{
    info!(query = %options.query_text, "Starting hybrid search");
    let analyzed_query = analyze_query(
        ai_provider.as_ref(),
        &options.query_text,
        options.prompts.analysis_system_prompt,
        options.prompts.analysis_user_prompt_template,
    )
    .await
    .map_err(SearchError::QueryAnalysis)?;

    // --- Sequential Retrieval ---
    // Augment AI-extracted keyphrases with raw keywords from the original query for robustness.
    let mut keyphrases_meta = analyzed_query.keyphrases.clone();
    keyphrases_meta.extend(options.query_text.split_whitespace().map(String::from));
    keyphrases_meta.sort();
    keyphrases_meta.dedup();

    let metadata_candidates = match provider
        .metadata_search(
            &analyzed_query.entities,
            &keyphrases_meta,
            options.owner_id.as_deref(),
            options.limit * 2,
        )
        .await
    {
        Ok(res) => {
            info!(
                "[hybrid_search] Metadata search returned {} candidates.",
                res.len()
            );
            res
        }
        Err(e) => {
            warn!("Metadata search task failed: {}", e);
            Vec::new()
        }
    };

    // Use the original query for the keyword search for robustness.
    let keyword_candidates = if options.use_keyword_search && !options.query_text.is_empty() {
        match provider
            .keyword_search(
                &options.query_text,
                options.limit * 2,
                options.owner_id.as_deref(),
                None,
            )
            .await
        {
            Ok(res) => {
                info!(
                    "[hybrid_search] Keyword search returned {} candidates.",
                    res.len()
                );
                res
            }
            Err(e) => {
                warn!("Keyword search task failed: {}", e);
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    let vector_candidates = if options.use_vector_search {
        let query_vector_result = generate_embeddings_batch(
            options.embedding_api_url,
            options.embedding_model,
            &[&options.query_text],
            options.embedding_api_key,
        )
        .await
        .map_err(SearchError::Embedding)
        .and_then(|mut vecs| {
            vecs.pop().ok_or_else(|| {
                SearchError::Embedding(PromptError::AiApi(
                    "Embedding API returned no vector".to_string(),
                ))
            })
        });

        match query_vector_result {
            Ok(query_vector) => {
                match provider
                    .vector_search(
                        query_vector,
                        options.limit * 2,
                        options.owner_id.as_deref(),
                        None,
                    )
                    .await
                {
                    Ok(res) => {
                        info!(
                            "[hybrid_search] Vector search returned {} candidates.",
                            res.len()
                        );
                        res
                    }
                    Err(e) => {
                        warn!("Vector search task failed: {}", e);
                        Vec::new()
                    }
                }
            }
            Err(e) => {
                warn!("Vector embedding generation failed: {}", e);
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    let ranked_parent_documents = reciprocal_rank_fusion(vec![
        metadata_candidates,
        vector_candidates,
        keyword_candidates,
    ]);

    // --- Step 4: Parse YAML and Expand into Contextual Chunks ---
    let mut contextual_chunks = Vec::new();
    for parent_doc in ranked_parent_documents {
        match serde_yaml::from_str::<YamlContent>(&parent_doc.description) {
            Ok(yaml_content) => {
                for section in yaml_content.sections {
                    let chunk_content = format!(
                        "## {}\n\n{}",
                        section.title,
                        section
                            .faqs
                            .iter()
                            .map(|faq| format!("### Q: {}\n\n{}", faq.question, faq.answer))
                            .collect::<Vec<_>>()
                            .join("\n\n")
                    );

                    contextual_chunks.push(SearchResult {
                        title: section.title.clone(),
                        link: format!("{}#{}", parent_doc.link, section.title.replace(' ', "_")),
                        description: chunk_content,
                        score: parent_doc.score, // Inherit score from parent
                    });
                }
            }
            Err(e) => {
                warn!(
                    "Failed to parse content of document '{}' as YAML, skipping. Error: {}",
                    parent_doc.link, e
                );
                // If parsing fails, we can still use the parent document as a fallback chunk.
                contextual_chunks.push(parent_doc);
            }
        }
    }

    // --- Step 5: Final Ranking and Truncation ---
    // The chunks are already roughly ordered by their parent's RRF score.
    let mut final_results = contextual_chunks;

    // --- Temporal Ranking Step ---
    if let Some(config) = &options.temporal_ranking_config {
        let is_temporal_query = analyzed_query
            .keyphrases
            .iter()
            .any(|phrase| config.keywords.iter().any(|kw| phrase.contains(*kw)));

        if is_temporal_query && !final_results.is_empty() {
            info!(
                "Temporal keyword detected. Re-ranking results by '{}'.",
                config.property_name
            );
            let mut ranked_results = temporally_rank_results(
                Arc::clone(&provider),
                final_results,
                config,
                options.owner_id.as_deref(),
            )
            .await;
            // As per the plan, truncate to the single most recent result for precision.
            ranked_results.truncate(1);
            final_results = ranked_results;
        }
    }

    final_results.truncate(options.limit as usize);

    if final_results.is_empty() {
        warn!(
            query = %options.query_text,
            "Hybrid search returned zero results from all sources."
        );
    }

    Ok(final_results)
}
