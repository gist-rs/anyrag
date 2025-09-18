use anyhow::Result;
use anyrag::errors::PromptError;
use anyrag::providers::ai::AiProvider;
use async_trait::async_trait;

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use turso::Database;

// --- Test Setup ---

/// A helper struct to manage database creation for each test.
pub struct TestSetup {
    pub db: Database,
}

impl TestSetup {
    pub async fn new() -> Result<Self> {
        // Use a unique in-memory DB for each test to ensure isolation.
        let db = turso::Builder::new_local(":memory:").build().await?;

        // Create a SqliteProvider to initialize the schema.
        // We create it with a new in-memory instance and then initialize.
        // The `db` instance from the builder is what we'll use in the test.
        let conn = db.connect()?;
        for statement in anyrag::providers::db::sqlite::sql::ALL_TABLE_CREATION_SQL {
            conn.execute(statement, ()).await?;
        }

        Ok(Self { db })
    }
}

// --- Mock AI Provider ---

#[derive(Clone, Debug)]
pub struct MockAiProvider {
    responses: Arc<Mutex<HashMap<String, String>>>,
    calls: Arc<Mutex<Vec<(String, String)>>>,
}

impl MockAiProvider {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(HashMap::new())),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Pre-programs a response for a specific prompt.
    /// The key should be a unique substring of the system prompt.
    pub fn add_response(&self, key: &str, response: &str) {
        let mut responses = self.responses.lock().unwrap();
        responses.insert(key.to_string(), response.to_string());
    }

    /// Retrieves the recorded calls for assertion.
    pub fn get_calls(&self) -> Vec<(String, String)> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl AiProvider for MockAiProvider {
    async fn generate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, PromptError> {
        let mut calls = self.calls.lock().unwrap();
        calls.push((system_prompt.to_string(), user_prompt.to_string()));

        let responses = self.responses.lock().unwrap();
        for (key, response) in responses.iter() {
            if system_prompt.contains(key) {
                return Ok(response.clone());
            }
        }

        Err(PromptError::AiApi(format!(
            "MockAiProvider: No response programmed for system prompt. Got: '{system_prompt}'"
        )))
    }
}
