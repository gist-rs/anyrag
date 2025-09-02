//! # Knowledge Base Route Handlers
//!
//! This module contains all the Axum handlers for interacting with the knowledge base,
//! including the main RAG search endpoint, embedding, exporting, and graph searches.

use super::{
    search::SearchRequest, wrap_response, AppError, AppState, DebugParams, PromptResponse,
};
use crate::auth::middleware::AuthenticatedUser;
use anyrag::{
    ingest::{export_for_finetuning, run_ingestion_pipeline},
    providers::ai::generate_embedding,
    search::hybrid_search,
    types::{ContentType, ExecutePromptOptions},
};
use axum::{
    extract::{Query, State},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info};
use turso::params;

// --- API Payloads for Knowledge Base ---

#[derive(Deserialize)]
pub struct KnowledgeIngestRequest {
    pub url: String,
}

#[derive(Serialize)]
pub struct KnowledgeIngestResponse {
    pub message: String,
    pub ingested_faqs: usize,
}

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
    let api_url = app_state
        .embeddings_api_url
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("EMBEDDINGS_API_URL not set")))?
        .clone();
    let model = app_state
        .embeddings_model
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("EMBEDDINGS_MODEL not set")))?
        .clone();

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

    let mut embedded_ids = Vec::new();
    for (doc_id, title, content) in docs_to_embed {
        let text_to_embed = format!("{title}. {content}");
        match generate_embedding(&api_url, &model, &text_to_embed).await {
            Ok(vector) => {
                let vector_bytes: &[u8] = unsafe {
                    std::slice::from_raw_parts(vector.as_ptr() as *const u8, vector.len() * 4)
                };
                if let Err(e) = conn
                    .execute(
                        "INSERT INTO document_embeddings (document_id, model_name, embedding) VALUES (?, ?, ?)",
                        params![doc_id.clone(), model.clone(), vector_bytes],
                    )
                    .await
                {
                    error!("Failed to insert embedding for document ID: {doc_id}. Error: {e}");
                } else {
                    info!("Successfully embedded document ID: {}", doc_id);
                    embedded_ids.push(doc_id);
                }
            }
            Err(e) => {
                error!("Failed to generate embedding for document ID: {doc_id}. Error: {e}");
            }
        }
    }

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
    info!(
        "User '{:?}' sending knowledge RAG search for query: '{}', limit: {}",
        owner_id, payload.query, limit
    );

    let api_url = app_state
        .embeddings_api_url
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("EMBEDDINGS_API_URL not set")))?;
    let model = app_state
        .embeddings_model
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("EMBEDDINGS_MODEL not set")))?;

    let query_vector = generate_embedding(api_url, model, &payload.query).await?;

    let search_results = hybrid_search(
        app_state.sqlite_provider.as_ref(),
        app_state.prompt_client.ai_provider.as_ref(),
        query_vector,
        &payload.query,
        owner_id.as_deref(),
        limit,
    )
    .await?;

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
            PromptResponse { text },
            debug_params,
            Some(debug_info),
        ));
    }

    info!("--> Synthesizing answer with context:\n{}", context);

    let options = ExecutePromptOptions {
        prompt: payload.query.clone(),
        content_type: Some(ContentType::Knowledge),
        context: Some(context.clone()),
        instruction: payload.instruction,
        ..Default::default()
    };
    let prompt_result = app_state
        .prompt_client
        .execute_prompt_with_options(options.clone())
        .await?;

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
            text: prompt_result.text,
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

/// Handler for the knowledge base ingestion pipeline.
pub async fn knowledge_ingest_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<KnowledgeIngestRequest>,
) -> Result<Json<super::ApiResponse<KnowledgeIngestResponse>>, AppError> {
    let owner_id = Some(user.0.id);
    info!(
        "Received knowledge ingest request for URL: {} by user {:?}",
        payload.url, owner_id
    );

    let ingested_count = run_ingestion_pipeline(
        &app_state.sqlite_provider.db,
        &*app_state.prompt_client.ai_provider,
        &payload.url,
        owner_id.as_deref(),
    )
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Knowledge ingestion failed: {e}")))?;
    let response = KnowledgeIngestResponse {
        message: "Knowledge ingestion pipeline completed successfully.".to_string(),
        ingested_faqs: ingested_count,
    };
    let debug_info = json!({ "url": payload.url, "owner_id": owner_id });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}
