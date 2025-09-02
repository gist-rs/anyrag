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

/// The shared application state, accessible from all request handlers.
#[derive(Clone)]
pub struct AppState {
    /// The application's configuration, loaded from `config.yml`.
    pub config: Arc<AppConfig>,
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
                Box::new(GeminiProvider::new(
                    provider_config.api_url.clone(),
                    api_key,
                )?)
            }
            "local" => Box::new(LocalAiProvider::new(
                provider_config.api_url.clone(),
                provider_config.api_key.clone(),
                Some(provider_config.model_name.clone()),
            )?),
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

    // The provider for local ingestion, embedding, and searching.
    let sqlite_provider = SqliteProvider::new(&config.db_url).await?;
    tracing::info!(db_path = %config.db_url, "Initialized local storage provider (SQLite).");
    // Ensure the database schema is up-to-date on startup.
    sqlite_provider.initialize_schema().await?;

    Ok(AppState {
        config: Arc::new(config),
        sqlite_provider: Arc::new(sqlite_provider),
        ai_providers: Arc::new(ai_providers),
        knowledge_graph: Arc::new(RwLock::new(MemoryKnowledgeGraph::new_memory())),
    })
}
