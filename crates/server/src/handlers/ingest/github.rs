use crate::auth::middleware::AuthenticatedUser;
use crate::handlers::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use anyrag::SearchResult;
use anyrag::{constants, ingest::Ingestor};
use anyrag_github::ingest::{search_examples, storage::StorageManager};
use anyrag_github::GithubIngestor;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

#[derive(Deserialize)]
pub struct IngestGitHubRequest {
    pub url: String,
    pub version: Option<String>,
}

#[derive(Serialize)]
pub struct IngestGitHubResponse {
    pub message: String,
    pub ingested_examples: usize,
    pub version: String,
}

#[derive(Deserialize)]
pub struct GetVersionedExamplesPath {
    pub repo_name: String,
    pub version: String,
}

#[derive(Deserialize)]
pub struct GetLatestExamplesPath {
    pub repo_name: String,
}

#[derive(Serialize)]
pub struct GetExamplesResponse {
    pub content: String,
}

#[derive(Deserialize)]
pub struct SearchExamplesRequest {
    pub query: String,
    pub repos: Vec<String>,
}

#[derive(Serialize)]
pub struct SearchExamplesResponse {
    pub results: Vec<SearchResult>,
}

/// Handler for ingesting code examples from a public GitHub repository.
/// This handler acts as a thin web layer, orchestrating the call to the
/// `anyrag-github` crate through the generic `Ingestor` trait.
pub async fn ingest_github_handler(
    State(app_state): State<AppState>,
    _user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<IngestGitHubRequest>,
) -> Result<Json<ApiResponse<IngestGitHubResponse>>, AppError> {
    info!("Received GitHub ingest request for URL: {}", payload.url);

    // 1. Instantiate the ingestor with configuration from the app state.
    // This decouples the server from the implementation details of the plugin.
    let ingestor = GithubIngestor::new(
        Some(app_state.config.embedding.api_url.clone()),
        Some(app_state.config.embedding.model_name.clone()),
        app_state.config.embedding.api_key.clone(),
    );

    // 2. Serialize the source information into a JSON string for the generic ingest method.
    let source_json = json!({
        "url": payload.url.clone(),
        "version": payload.version.clone()
    })
    .to_string();

    // 3. Call the generic ingest method from the trait.
    let ingest_result = ingestor
        .ingest(&source_json, None) // owner_id is not used for github ingestion
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("GitHub ingestion failed: {e}")))?;

    // 4. Parse the version from the result source for the response.
    let ingested_version = ingest_result
        .source
        .rsplit_once('#')
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // 5. Construct the final HTTP response.
    let response = IngestGitHubResponse {
        message: "GitHub ingestion pipeline completed successfully.".to_string(),
        ingested_examples: ingest_result.documents_added,
        version: ingested_version,
    };
    let debug_info = json!({ "url": payload.url, "version": payload.version });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

/// Handler for retrieving a consolidated Markdown file of examples for a specific repository version.
pub async fn get_versioned_examples_handler(
    State(_app_state): State<AppState>,
    Path(path): Path<GetVersionedExamplesPath>,
    debug_params: Query<DebugParams>,
) -> Result<Json<ApiResponse<GetExamplesResponse>>, AppError> {
    info!(
        "Received request for examples for repo '{}', version '{}'",
        path.repo_name, path.version
    );

    let storage_manager = StorageManager::new(constants::GITHUB_DB_DIR).await?;

    let examples = storage_manager
        .get_examples(&path.repo_name, &path.version)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to retrieve examples: {}", e)))?;

    if examples.is_empty() {
        let response = GetExamplesResponse {
            content: format!(
                "# No examples found for repository '{}' version '{}'.",
                path.repo_name, path.version
            ),
        };
        return Ok(wrap_response(response, debug_params, None));
    }

    let markdown_content = examples
        .iter()
        .map(|ex| {
            format!(
                "## `{}`\n**Source:** `{}` (`{}`)\n\n```rust\n{}\n```\n",
                ex.example_handle, ex.source_file, ex.source_type, ex.content
            )
        })
        .collect::<Vec<String>>()
        .join("---\n");

    let response = GetExamplesResponse {
        content: markdown_content,
    };

    let debug_info = json!({ "repo_name": path.repo_name, "version": path.version, "example_count": examples.len() });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

/// Handler for retrieving examples for the latest version of a repository.
pub async fn get_latest_examples_handler(
    State(_app_state): State<AppState>,
    Path(path): Path<GetLatestExamplesPath>,
    debug_params: Query<DebugParams>,
) -> Result<Json<ApiResponse<GetExamplesResponse>>, AppError> {
    info!(
        "Received request for latest examples for repo '{}'",
        path.repo_name
    );

    let storage_manager = StorageManager::new(constants::GITHUB_DB_DIR).await?;

    let latest_version = storage_manager
        .get_latest_version(&path.repo_name)
        .await?
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Could not determine latest version for repo '{}'",
                path.repo_name
            ))
        })?;

    info!(
        "Found latest version for repo '{}': {}",
        path.repo_name, latest_version
    );

    let examples = storage_manager
        .get_examples(&path.repo_name, &latest_version)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to retrieve examples: {}", e)))?;

    if examples.is_empty() {
        let response = GetExamplesResponse {
            content: format!(
                "# No examples found for repository '{}' version '{}'.",
                path.repo_name, latest_version
            ),
        };
        return Ok(wrap_response(response, debug_params, None));
    }

    let markdown_content = examples
        .iter()
        .map(|ex| {
            format!(
                "## `{}`\n**Source:** `{}` (`{}`)\n\n```rust\n{}\n```\n",
                ex.example_handle, ex.source_file, ex.source_type, ex.content
            )
        })
        .collect::<Vec<String>>()
        .join("---\n");

    let response = GetExamplesResponse {
        content: markdown_content,
    };

    let debug_info = json!({ "repo_name": path.repo_name, "version_retrieved": latest_version, "example_count": examples.len() });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}

/// Handler for the RAG search endpoint for code examples.
pub async fn search_examples_handler(
    State(app_state): State<AppState>,
    _user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<SearchExamplesRequest>,
) -> Result<Json<ApiResponse<SearchExamplesResponse>>, AppError> {
    info!(
        "Received example search request for query: '{}' in repos: {:?}",
        payload.query, payload.repos
    );

    let task_name = "query_analysis";
    let task_config = app_state.tasks.get(task_name).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "Configuration for task '{}' not found.",
            task_name
        ))
    })?;
    let provider_name = &task_config.provider;
    let ai_provider = app_state
        .ai_providers
        .get(provider_name)
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Provider '{}' not found in providers map.",
                provider_name
            ))
        })?
        .clone();
    let embedding_api_url = &app_state.config.embedding.api_url;
    let embedding_model = &app_state.config.embedding.model_name;
    let embedding_api_key = app_state.config.embedding.api_key.as_deref();

    let storage_manager = StorageManager::new(constants::GITHUB_DB_DIR).await?;

    let search_results = search_examples(
        &storage_manager,
        &payload.query,
        &payload.repos,
        std::sync::Arc::from(ai_provider),
        embedding_api_url,
        embedding_model,
        embedding_api_key,
    )
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Example search failed: {}", e)))?;

    let response = SearchExamplesResponse {
        results: search_results,
    };
    let debug_info = json!({ "query": payload.query, "repos": payload.repos });
    Ok(wrap_response(response, debug_params, Some(debug_info)))
}
