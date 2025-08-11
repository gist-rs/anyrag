use crate::{
    errors::PromptError,
    prompts::{DEFAULT_QUERY_SYSTEM_PROMPT, DEFAULT_QUERY_USER_PROMPT},
    providers::{ai::AiProvider, db::bigquery::BigQueryProvider, db::storage::Storage},
    rerank::Rerankable,
};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug};

/// A client for executing natural language prompts against a storage provider.
///
/// This client orchestrates the process of converting a prompt into a SQL query
/// using a configurable AI provider and then executing that query against a
/// configurable storage provider.
pub struct PromptClient {
    pub ai_provider: Box<dyn AiProvider>,
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

/// Represents different content types to guide prompt generation.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    Rss,
    Sql,
    Json,
    Text,
}

impl ContentType {
    /// Returns the appropriate system and user prompt templates for the content type.
    pub fn get_prompt_templates(&self) -> (&'static str, &'static str) {
        // TODO: Move these specific prompts to prompts.rs
        const RSS_QUERY_SYSTEM_PROMPT: &str = "You are an AI assistant that specializes in analyzing and summarizing content from RSS feeds. Answer the user's question based on the provided article snippets.";
        const RSS_QUERY_USER_PROMPT: &str =
            "# User Question\n{prompt}\n\n# Article Content\n{context}";

        match self {
            ContentType::Rss => (RSS_QUERY_SYSTEM_PROMPT, RSS_QUERY_USER_PROMPT),
            // Default to standard SQL prompts for other types for now.
            ContentType::Sql | ContentType::Json | ContentType::Text => {
                (DEFAULT_QUERY_SYSTEM_PROMPT, DEFAULT_QUERY_USER_PROMPT)
            }
        }
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
    /// An optional hint about the content type to guide prompt selection.
    #[serde(default)]
    pub content_type: Option<ContentType>,
    /// The content to be used in the prompt, when `content_type` is provided.
    #[serde(default)]
    pub context: Option<String>,

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
    /// A template for the user prompt for the final formatting step.
    /// Available placeholders: `{prompt}`, `{instruction}`, `{content}`
    #[serde(default)]
    pub format_user_prompt_template: Option<String>,
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

    /// A helper to build and set a `SqliteProvider` as the storage provider.
    pub async fn sqlite_storage(mut self, db_path: &str) -> Result<Self, PromptError> {
        let provider = crate::providers::db::sqlite::SqliteProvider::new(db_path).await?;
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

/// A search result from any search provider (vector, keyword, etc.).
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SearchResult {
    pub title: String,
    pub link: String,
    pub description: String,
    /// A generic score. For vector search, lower is better (distance). For RRF/LLM, higher is better.
    pub score: f64,
}

impl Rerankable for SearchResult {
    fn get_title(&self) -> &str {
        &self.title
    }

    fn get_link(&self) -> &str {
        &self.link
    }

    fn get_description(&self) -> &str {
        &self.description
    }
}
