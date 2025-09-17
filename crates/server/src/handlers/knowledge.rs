//! # Knowledge Base Route Handlers
//!
//! This module contains all the Axum handlers for interacting with the knowledge base,
//! including the main RAG search endpoint, embedding, exporting, and graph searches.

use super::{
    search::SearchRequest, wrap_response, AppError, AppState, DebugParams, PromptResponse,
};
use crate::auth::middleware::AuthenticatedUser;
use anyrag::{
    constants,
    ingest::export_for_finetuning,
    providers::{ai::generate_embeddings_batch, db::sqlite::SqliteProvider},
    search::{hybrid_search, HybridSearchOptions, HybridSearchPrompts},
    types::{ContentType, ExecutePromptOptions, PromptClientBuilder},
};
use axum::{
    extract::{Query, State},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{error, info};
use turso::params;

// --- API Payloads for Knowledge Base ---

#[derive(Deserialize, Debug)]
pub struct EmbedNewRequest {
    pub limit: Option<usize>,
}

#[derive(Serialize, Debug)]
pub struct EmbedNewResponse {
    message: String,
    embedded_articles: usize,
}

#[derive(Deserialize)]
pub struct KnowledgeGraphSearchRequest {
    pub subject: String,
    pub predicate: String,
}

#[derive(Serialize)]
pub struct KnowledgeGraphSearchResponse {
    pub object: Option<String>,
}

// --- Knowledge Base Handlers ---

/// Handler for embedding new, unprocessed documents in the knowledge base.
pub async fn embed_new_handler(
    State(app_state): State<AppState>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<EmbedNewRequest>,
) -> Result<Json<super::ApiResponse<EmbedNewResponse>>, AppError> {
    let limit = payload.limit.unwrap_or(20);
    info!("Received request to embed up to {limit} new documents.");

    // Get embedding config from AppState
    let api_url = &app_state.config.embedding.api_url;
    let model = &app_state.config.embedding.model_name;
    let api_key = app_state.config.embedding.api_key.as_deref();

    let conn = app_state.sqlite_provider.db.connect()?;
    let sql = format!(
        "
        SELECT d.id, d.title, d.content
        FROM documents d
        LEFT JOIN document_embeddings de ON d.id = de.document_id
        WHERE de.id IS NULL
        LIMIT {limit}
    "
    );
    let mut stmt = conn.prepare(&sql).await?;
    let mut rows = stmt.query(()).await?;

    let mut docs_to_embed = Vec::new();
    while let Some(row) = rows.next().await? {
        let id: String = row.get(0)?;
        let title: String = row.get(1)?;
        let content: String = row.get(2)?;
        docs_to_embed.push((id, title, content));
    }

    let embed_count = docs_to_embed.len();
    info!("Found {embed_count} documents to embed.");

    if docs_to_embed.is_empty() {
        let response = EmbedNewResponse {
            message: "No new documents to embed.".to_string(),
            embedded_articles: 0,
        };
        let debug_info = json!({ "limit": limit, "found": 0 });
        return Ok(wrap_response(response, debug_params, Some(debug_info)));
    }

    // 1. Prepare texts for batch embedding
    let texts_to_embed: Vec<String> = docs_to_embed
        .iter()
        .map(|(_, title, content)| format!("{title}. {content}"))
        .collect();
    let text_slices: Vec<&str> = texts_to_embed.iter().map(AsRef::as_ref).collect();

    // 2. Call the batch embedding function
    let embeddings = match generate_embeddings_batch(api_url, model, &text_slices, api_key).await {
        Ok(vectors) => vectors,
        Err(e) => {
            error!("Batch embedding generation failed: {e}");
            return Err(AppError::Embedding(
                anyrag::ingest::EmbeddingError::Embedding(e),
            ));
        }
    };

    if docs_to_embed.len() != embeddings.len() {
        error!(
            "Mismatch between number of documents ({}) and embeddings received ({}). Aborting.",
            docs_to_embed.len(),
            embeddings.len()
        );
        return Err(AppError::Internal(anyhow::anyhow!(
            "Embedding count mismatch."
        )));
    }

    // 3. Store the embeddings in a transaction
    let mut embedded_ids = Vec::new();
    conn.execute("BEGIN TRANSACTION", ()).await?;
    let mut stmt = conn
        .prepare(
            "INSERT INTO document_embeddings (document_id, model_name, embedding) VALUES (?, ?, ?)",
        )
        .await?;

    for ((doc_id, _, _), vector) in docs_to_embed.iter().zip(embeddings) {
        let vector_bytes: &[u8] =
            unsafe { std::slice::from_raw_parts(vector.as_ptr() as *const u8, vector.len() * 4) };

        if let Err(e) = stmt
            .execute(params![doc_id.clone(), model.clone(), vector_bytes])
            .await
        {
            error!("Failed to insert embedding for document ID: {doc_id}. Error: {e}");
        } else {
            embedded_ids.push(doc_id.clone());
        }
    }
    conn.execute("COMMIT", ()).await?;

    let success_count = embedded_ids.len();
    let response = EmbedNewResponse {
        message: format!(
            "Successfully processed embeddings for {success_count} of {embed_count} documents."
        ),
        embedded_articles: success_count,
    };
    let debug_info = json!({ "limit": limit, "found": embed_count, "embedded_ids": embedded_ids });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

/// Handler for exporting the knowledge base for fine-tuning.
pub async fn knowledge_export_handler(
    State(app_state): State<AppState>,
) -> Result<String, AppError> {
    info!("Received request to export knowledge base for fine-tuning.");
    let jsonl_data = export_for_finetuning(&app_state.sqlite_provider.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Knowledge export failed: {e}")))?;
    Ok(jsonl_data)
}

/// Handler for the primary RAG search endpoint against the knowledge base.
#[axum::debug_handler]
pub async fn knowledge_search_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<super::ApiResponse<PromptResponse>>, AppError> {
    let owner_id = Some(user.0.id);
    let limit = payload.limit.unwrap_or(5);

    // --- Dynamic DB Connection ---
    let sqlite_provider = if let Some(db_name) = &payload.db {
        let db_path = format!("{}/{}.db", constants::DB_DIR, db_name);
        info!("Connecting to dynamic database: {}", db_path);
        let provider = SqliteProvider::new(&db_path).await?;
        Arc::new(provider)
    } else {
        app_state.sqlite_provider.clone()
    };

    info!(
        "User '{:?}' sending knowledge RAG search for query: '{}', limit: {}",
        owner_id, payload.query, limit
    );

    // --- Get AI provider for query analysis ---
    let task_name = "query_analysis";
    let task_config = app_state.tasks.get(task_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("Task '{task_name}' not found in config"))
    })?;

    let analysis_provider = if let Some(model_name) = &payload.model {
        info!("Model override requested for analysis: {}", model_name);
        let provider_name = app_state
            .config
            .providers
            .iter()
            .find(|(_, p)| p.model_name == *model_name)
            .map(|(name, _)| name)
            .ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!(
                    "Model '{model_name}' not found in any configured provider"
                ))
            })?;
        app_state.ai_providers.get(provider_name).ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("Provider '{provider_name}' not found"))
        })?
    } else {
        let provider_name = &task_config.provider;
        app_state.ai_providers.get(provider_name).ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("Provider '{provider_name}' not found"))
        })?
    };
    let ai_provider = Arc::from(analysis_provider.clone());

    let temporal_keywords: Vec<&str>;
    let temporal_ranking_config = if let Some(config) = &app_state.config.temporal_reasoning {
        temporal_keywords = config.keywords.iter().map(|s| s.as_str()).collect();
        Some(anyrag::search::TemporalRankingConfig {
            keywords: &temporal_keywords,
            property_name: &config.property_name,
        })
    } else {
        None
    };

    let search_options = HybridSearchOptions {
        query_text: payload.query.clone(),
        owner_id,
        limit,
        prompts: HybridSearchPrompts {
            analysis_system_prompt: &task_config.system_prompt,
            analysis_user_prompt_template: &task_config.user_prompt,
        },
        use_keyword_search: true,
        use_vector_search: true,
        embedding_api_url: &app_state.config.embedding.api_url,
        embedding_model: &app_state.config.embedding.model_name,
        embedding_api_key: app_state.config.embedding.api_key.as_deref(),
        temporal_ranking_config,
    };

    let search_results =
        hybrid_search(sqlite_provider.clone(), ai_provider, search_options).await?;

    let kg_fact = if payload.use_knowledge_graph.unwrap_or(false) {
        info!("Knowledge graph search is enabled for this request.");
        let kg = app_state
            .knowledge_graph
            .read()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to acquire KG read lock")))?;

        let predicate = "role";
        kg.get_fact_as_of(&payload.query, predicate, Utc::now())
            .ok()
            .flatten()
    } else {
        None
    };

    let mut context_parts = Vec::new();

    if let Some(fact) = kg_fact {
        info!("Found definitive fact in Knowledge Graph: {}", fact);
        context_parts.push(format!("Definitive Answer from Knowledge Graph: {fact}."));
    }

    if !search_results.is_empty() {
        let articles_context = search_results
            .iter()
            .map(|result| result.description.clone())
            .collect::<Vec<String>>()
            .join("\n\n---\n\n");

        if !context_parts.is_empty() {
            context_parts.push(format!(
                "Additional Context from Documents:\n{articles_context}"
            ));
        } else {
            context_parts.push(articles_context);
        }
    }

    let context = context_parts.join("\n\n");

    if context.is_empty() {
        let text = "I could not find any relevant information to answer your question.".to_string();
        let debug_info =
            json!({ "query": payload.query, "limit": limit, "status": "No results found" });
        return Ok(wrap_response(
            PromptResponse {
                text: Value::String(text),
            },
            debug_params,
            Some(debug_info),
        ));
    }

    info!("--> Synthesizing answer with context:\n{}", context);

    // --- Get AI provider for RAG synthesis ---
    let task_name = "rag_synthesis";
    let task_config = app_state.tasks.get(task_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("Task '{task_name}' not found in config"))
    })?;

    let synthesis_provider = if let Some(model_name) = &payload.model {
        info!("Model override requested for synthesis: {}", model_name);
        // We can reuse the same logic as for the analysis provider
        let provider_name = app_state
            .config
            .providers
            .iter()
            .find(|(_, p)| p.model_name == *model_name)
            .map(|(name, _)| name)
            .ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!(
                    "Model '{model_name}' not found in any configured provider"
                ))
            })?;
        app_state.ai_providers.get(provider_name).ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("Provider '{provider_name}' not found"))
        })?
    } else {
        let provider_name = &task_config.provider;
        app_state.ai_providers.get(provider_name).ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("Provider '{provider_name}' not found"))
        })?
    };

    // Manually combine prompt and instruction for the final synthesis step.
    // This is safer than modifying the library's prompt templates or logic.
    let final_prompt =
        if let Some(instruction) = payload.instruction.as_deref().filter(|s| !s.is_empty()) {
            // The instruction is appended to the prompt to guide the final answer synthesis.
            // The extra newlines help separate it for the LLM.
            format!("{}\n\n{}", payload.query, instruction)
        } else {
            payload.query.clone()
        };

    let mut options = ExecutePromptOptions {
        prompt: final_prompt, // Use the combined prompt
        content_type: Some(ContentType::Knowledge),
        context: Some(context.clone()),
        instruction: None, // It's now part of the prompt
        ..Default::default()
    };

    // Apply prompts from config
    options.system_prompt_template = Some(task_config.system_prompt.clone());
    options.user_prompt_template = Some(task_config.user_prompt.clone());

    let client = PromptClientBuilder::new()
        .ai_provider(synthesis_provider.clone())
        .storage_provider(Box::new(sqlite_provider.as_ref().clone()))
        .build()?;

    let prompt_result = client.execute_prompt_with_options(options.clone()).await?;

    let debug_info = if debug_params.debug.unwrap_or(false) {
        Some(json!({
            "options": options,
            "retrieved_context": context,
            "final_candidate_count": search_results.len()
        }))
    } else {
        None
    };
    Ok(wrap_response(
        PromptResponse {
            text: Value::String(prompt_result.text),
        },
        debug_params,
        debug_info,
    ))
}

/// Handler for performing a direct search on the knowledge graph.
pub async fn knowledge_graph_search_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<KnowledgeGraphSearchRequest>,
) -> Result<Json<super::ApiResponse<KnowledgeGraphSearchResponse>>, AppError> {
    info!(
        "Received knowledge graph search for subject: '{}', predicate: '{}'",
        payload.subject, payload.predicate
    );

    let object = {
        let kg = app_state
            .knowledge_graph
            .read()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to acquire KG read lock")))?;
        kg.get_fact_as_of(&payload.subject, &payload.predicate, Utc::now())
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Knowledge graph query failed: {e}")))?
    };

    let response = KnowledgeGraphSearchResponse { object };

    Ok(Json(super::ApiResponse {
        debug: None,
        result: response,
    }))
}
