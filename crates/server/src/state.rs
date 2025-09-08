//! # Application State
//!
//! This module defines the shared application state (`AppState`) and the logic
//! for building it at startup. The `AppState` holds all shared resources, such
//! as the configuration, database connections, and instantiated AI provider clients,
//! making them accessible to all request handlers.

use crate::config::AppConfig;
use anyrag::{
    graph::types::MemoryKnowledgeGraph,
    providers::{
        ai::{gemini::GeminiProvider, local::LocalAiProvider, AiProvider},
        db::sqlite::SqliteProvider,
    },
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// A fully resolved task configuration with non-optional fields.
#[derive(Clone, Debug)]
pub struct ResolvedTask {
    pub provider: String,
    pub system_prompt: String,
    pub user_prompt: String,
}

/// The shared application state, accessible from all request handlers.
#[derive(Clone)]
pub struct AppState {
    /// The application's configuration, loaded from `config.yml`.
    pub config: Arc<AppConfig>,
    /// A map of fully resolved tasks, ready for use by handlers.
    pub tasks: Arc<HashMap<String, ResolvedTask>>,
    /// The primary database provider for local storage and knowledge base.
    pub sqlite_provider: Arc<SqliteProvider>,
    /// A map of instantiated AI providers, keyed by their name from the config.
    pub ai_providers: Arc<HashMap<String, Box<dyn AiProvider>>>,
    /// An in-memory knowledge graph for time-sensitive, precise data.
    pub knowledge_graph: Arc<RwLock<MemoryKnowledgeGraph>>,
}

/// Builds the shared application state from the configuration.
///
/// This function initializes all necessary services:
/// - It instantiates an AI provider client for each entry in the `providers`
///   section of the configuration.
/// - It sets up the connection to the SQLite database.
/// - It initializes an in-memory knowledge graph.
pub async fn build_app_state(config: AppConfig) -> anyhow::Result<AppState> {
    // Create a map of AI provider instances from the configuration.
    let mut ai_providers = HashMap::new();
    for (name, provider_config) in &config.providers {
        let provider: Box<dyn AiProvider> = match provider_config.provider.as_str() {
            "gemini" => {
                let api_key = provider_config.api_key.clone().ok_or_else(|| {
                    anyhow::anyhow!("api_key is required for gemini provider '{name}'")
                })?;
                // If api_url is not provided in config, construct it from the model name.
                let api_url = provider_config.api_url.clone().unwrap_or_else(|| {
                    format!(
                        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
                        provider_config.model_name
                    )
                });
                Box::new(GeminiProvider::new(api_url, api_key)?)
            }
            "local" => {
                // For local providers, the URL is always required.
                let api_url = provider_config.api_url.clone().ok_or_else(|| {
                    anyhow::anyhow!(
                        "api_url is required for local provider '{name}'. Please set LOCAL_AI_API_URL in your .env file."
                    )
                })?;
                Box::new(LocalAiProvider::new(
                    api_url,
                    provider_config.api_key.clone(),
                    Some(provider_config.model_name.clone()),
                )?)
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported AI provider type '{}' for provider '{}'",
                    provider_config.provider,
                    name
                ));
            }
        };
        ai_providers.insert(name.clone(), provider);
    }

    // Validate and resolve all tasks from the configuration.
    // The config loading ensures that all default tasks have their fields populated,
    // so we can safely unwrap them here. A panic here indicates a misconfiguration
    // in the static defaults or a malformed config file.
    let mut resolved_tasks = HashMap::new();
    for (name, task_config) in &config.tasks {
        let provider = task_config.provider.clone().ok_or_else(|| {
            anyhow::anyhow!("Resolved task '{name}' is missing required 'provider' field")
        })?;
        let system_prompt = task_config.system_prompt.clone().ok_or_else(|| {
            anyhow::anyhow!("Resolved task '{name}' is missing required 'system_prompt' field")
        })?;
        let user_prompt = task_config.user_prompt.clone().ok_or_else(|| {
            anyhow::anyhow!("Resolved task '{name}' is missing required 'user_prompt' field")
        })?;

        resolved_tasks.insert(
            name.clone(),
            ResolvedTask {
                provider,
                system_prompt,
                user_prompt,
            },
        );
    }

    // The provider for local ingestion, embedding, and searching.
    let sqlite_provider = SqliteProvider::new(&config.db_url).await?;
    tracing::info!(db_path = %config.db_url, "Initialized local storage provider (SQLite).");
    // Ensure the database schema is up-to-date on startup.
    sqlite_provider.initialize_schema().await?;

    Ok(AppState {
        config: Arc::new(config),
        tasks: Arc::new(resolved_tasks),
        sqlite_provider: Arc::new(sqlite_provider),
        ai_providers: Arc::new(ai_providers),
        knowledge_graph: Arc::new(RwLock::new(MemoryKnowledgeGraph::new_memory())),
    })
}
