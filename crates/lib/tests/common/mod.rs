//! # Common Test Utilities
//!
//! This module provides shared utilities for testing, such as mock servers
//! and mock providers, to ensure tests are isolated and repeatable.

use anyrag::providers::ai::AiProvider;
use anyrag::providers::db::storage::Storage;
use async_trait::async_trait;
use dotenvy::dotenv;
use gcp_bigquery_client::model::table_schema::TableSchema;
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
        Ok(Arc::new(TableSchema::new(vec![])))
    }
}
