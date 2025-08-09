use crate::{
    errors::PromptError,
    providers::{ai::AiProvider, db::bigquery::BigQueryProvider, db::storage::Storage},
};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug};

/// A client for executing natural language prompts against a storage provider.
///
/// This client orchestrates the process of converting a prompt into a SQL query
/// using a configurable AI provider and then executing that query against a
/// configurable storage provider.
pub struct PromptClient {
    pub(crate) ai_provider: Box<dyn AiProvider>,
    pub(crate) storage_provider: Box<dyn Storage>,
}

impl Debug for PromptClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PromptClient")
            .field("ai_provider", &self.ai_provider)
            .field("storage_provider", &self.storage_provider)
            .finish()
    }
}

/// Options for executing a prompt.
///
/// This struct encapsulates all the parameters for prompt execution,
/// allowing for fine-grained control over the AI and storage providers.
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ExecutePromptOptions {
    /// The natural language prompt to be executed.
    pub prompt: String,
    /// The name of the table to be queried (e.g., "project.dataset.table").
    #[serde(default)]
    pub table_name: Option<String>,
    /// An instruction for the AI on how to format the final response.
    #[serde(default)]
    pub instruction: Option<String>,
    /// A key to use for aliasing the result column in the SQL query.
    #[serde(default)]
    pub answer_key: Option<String>,
    /// A template for the system prompt to override the default.
    #[serde(default)]
    pub system_prompt_template: Option<String>,
    /// A template for the user prompt to override the default.
    /// Placeholders like `{context}` and `{prompt}` will be replaced.
    #[serde(default)]
    pub user_prompt_template: Option<String>,
    /// A template for the system prompt for the final formatting step.
    #[serde(default)]
    pub format_system_prompt_template: Option<String>,
}

/// A builder for creating `PromptClient` instances.
///
/// This builder facilitates the creation of a `PromptClient` by allowing
/// for the configuration of AI and storage providers.
#[derive(Default)]
pub struct PromptClientBuilder {
    ai_provider: Option<Box<dyn AiProvider>>,
    storage_provider: Option<Box<dyn Storage>>,
}

impl PromptClientBuilder {
    /// Creates a new `PromptClientBuilder`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the AI provider instance.
    pub fn ai_provider(mut self, ai_provider: Box<dyn AiProvider>) -> Self {
        self.ai_provider = Some(ai_provider);
        self
    }

    /// Sets the storage provider instance.
    pub fn storage_provider(mut self, storage_provider: Box<dyn Storage>) -> Self {
        self.storage_provider = Some(storage_provider);
        self
    }

    /// A helper to build and set a `BigQueryProvider` as the storage provider.
    pub async fn bigquery_storage(mut self, project_id: String) -> Result<Self, PromptError> {
        let provider = BigQueryProvider::new(project_id).await?;
        self.storage_provider = Some(Box::new(provider));
        Ok(self)
    }

    /// Builds the `PromptClient`.
    ///
    /// This method consumes the builder and returns a `Result` containing
    /// either a configured `PromptClient` or a `PromptError` if configuration
    /// is incomplete.
    pub fn build(self) -> Result<PromptClient, PromptError> {
        let ai_provider = self.ai_provider.ok_or(PromptError::MissingAiProvider)?;
        let storage_provider = self
            .storage_provider
            .ok_or(PromptError::MissingStorageProvider)?;

        Ok(PromptClient {
            ai_provider,
            storage_provider,
        })
    }
}
