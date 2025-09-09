#![allow(dead_code)]
//! # Common Test Utilities
//!
//! This module provides shared utilities for testing, such as mock servers
//! and mock providers, to ensure tests are isolated and repeatable.

use anyrag::providers::ai::{gemini::GeminiProvider, local::LocalAiProvider, AiProvider};
use anyrag::providers::db::storage::Storage;
use anyrag::types::TableSchema;
use async_trait::async_trait;
use dotenvy::dotenv;
use std::env;
use std::fmt::Debug;
use std::sync::{Arc, Once, RwLock};

#[cfg(test)]
static INIT: Once = Once::new();

/// Initializes the tracing subscriber and loads .env for tests.
#[cfg(test)]
pub fn setup_tracing() {
    INIT.call_once(|| {
        dotenv().ok();
        tracing_subscriber::fmt::init();
    });
}

// --- Mock AI Provider for Logic Testing ---
#[derive(Clone, Debug)]
pub struct MockAiProvider {
    pub call_history: Arc<RwLock<Vec<(String, String)>>>,
    pub responses: Arc<RwLock<Vec<String>>>,
}

impl MockAiProvider {
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            call_history: Arc::new(RwLock::new(Vec::new())),
            responses: Arc::new(RwLock::new(responses.into_iter().rev().collect())),
        }
    }
}

#[async_trait]
impl AiProvider for MockAiProvider {
    async fn generate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, anyrag::PromptError> {
        self.call_history
            .write()
            .unwrap()
            .push((system_prompt.to_string(), user_prompt.to_string()));

        if let Some(response) = self.responses.write().unwrap().pop() {
            Ok(response)
        } else {
            Ok("Default mock response".to_string())
        }
    }
}

// --- Mock Storage Provider for Testing ---
#[derive(Clone, Debug)]
pub struct MockStorageProvider;

#[async_trait]
impl Storage for MockStorageProvider {
    fn name(&self) -> &str {
        "MockDB"
    }
    fn language(&self) -> &str {
        "SQL"
    }
    async fn execute_query(&self, _query: &str) -> Result<String, anyrag::PromptError> {
        Ok("[]".to_string())
    }
    async fn get_table_schema(
        &self,
        _table_name: &str,
    ) -> Result<Arc<TableSchema>, anyrag::PromptError> {
        Ok(Arc::new(TableSchema::default()))
    }

    async fn list_tables(&self) -> Result<Vec<String>, anyrag::PromptError> {
        Ok(vec!["mock_table".to_string()])
    }
}

/// Creates a "real" AI provider based on environment variables.
///
/// This helper reads the `AI_PROVIDER` variable to decide whether to instantiate
/// a `GeminiProvider` or a `LocalAiProvider`, using other environment variables
/// for configuration. Panics if required variables are not set.
pub fn create_real_ai_provider() -> Box<dyn AiProvider> {
    let provider_name = env::var("AI_PROVIDER").unwrap_or_else(|_| "local".to_string());
    let api_key = env::var("AI_API_KEY").ok();
    let model = env::var("AI_MODEL").ok();

    match provider_name.as_str() {
        "gemini" => {
            let api_url =
                env::var("AI_API_URL").expect("AI_API_URL is required for gemini provider");
            let key = api_key.expect("AI_API_KEY is required for the gemini provider");
            Box::new(GeminiProvider::new(api_url, key).expect("Failed to create GeminiProvider"))
        }
        "local" => {
            let api_url = env::var("LOCAL_AI_API_URL")
                .or_else(|_| env::var("AI_API_URL"))
                .expect("LOCAL_AI_API_URL or AI_API_URL must be set for local provider");
            Box::new(
                LocalAiProvider::new(api_url, api_key, model)
                    .expect("Failed to create LocalAiProvider"),
            )
        }
        _ => panic!("Unsupported AI provider specified: {provider_name}"),
    }
}
