#[cfg(feature = "rss")]
use crate::prompts::tasks::{RSS_SUMMARIZATION_SYSTEM_PROMPT, RSS_SUMMARIZATION_USER_PROMPT};
#[cfg(feature = "bigquery")]
use crate::providers::db::bigquery::BigQueryProvider;
use crate::{
    constants,
    errors::PromptError,
    prompts::{
        core::DEFAULT_QUERY_SYSTEM_PROMPT,
        knowledge::{KNOWLEDGE_RAG_SYSTEM_PROMPT, KNOWLEDGE_RAG_USER_PROMPT},
        tasks::QUERY_GENERATION_USER_PROMPT,
    },
    providers::{ai::AiProvider, db::storage::Storage},
    rerank::Rerankable,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    #[cfg(feature = "rss")]
    Rss,
    Sql,
    Json,
    Text,
    Knowledge,
}

impl ContentType {
    /// Returns the appropriate system and user prompt templates for the content type.
    pub fn get_prompt_templates(&self) -> (&'static str, &'static str) {
        match self {
            #[cfg(feature = "rss")]
            ContentType::Rss => (
                RSS_SUMMARIZATION_SYSTEM_PROMPT,
                RSS_SUMMARIZATION_USER_PROMPT,
            ),
            ContentType::Knowledge => (KNOWLEDGE_RAG_SYSTEM_PROMPT, KNOWLEDGE_RAG_USER_PROMPT),
            // Default to standard SQL prompts for other types for now.
            ContentType::Sql | ContentType::Json | ContentType::Text => {
                (DEFAULT_QUERY_SYSTEM_PROMPT, QUERY_GENERATION_USER_PROMPT)
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
    /// For BigQuery, the project ID. If provided, the query will run against BigQuery instead of the default provider.
    #[serde(default)]
    pub project_id: Option<String>,
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

/// The result of a successful prompt execution, including debug information.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PromptResult {
    /// The final, user-facing result, typically a natural language answer.
    pub text: String,
    /// The generated SQL query that was executed against the database.
    #[serde(default)]
    pub generated_sql: Option<String>,
    /// The raw, unprocessed result from the database query.
    #[serde(default)]
    pub database_result: Option<String>,
    /// The system prompt sent to the AI for query generation.
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// The user prompt sent to the AI for query generation.
    #[serde(default)]
    pub user_prompt: Option<String>,
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
    #[cfg(feature = "bigquery")]
    pub async fn bigquery_storage(mut self, project_id: String) -> Result<Self, PromptError> {
        let provider = BigQueryProvider::new(project_id).await?;
        self.storage_provider = Some(Box::new(provider));
        Ok(self)
    }

    /// A helper to build and set a `BigQueryProvider` as the storage provider.
    #[cfg(not(feature = "bigquery"))]
    pub async fn bigquery_storage(self, _project_id: String) -> Result<Self, PromptError> {
        Err(PromptError::BigQueryFeatureNotEnabled)
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
        let ai_provider = self.ai_provider.ok_or_else(|| {
            PromptError::MissingAiProvider(
                "AI provider has not been set in the builder.".to_string(),
            )
        })?;
        let storage_provider = self.storage_provider.ok_or_else(|| {
            PromptError::MissingStorageProvider(
                "Storage provider has not been set in the builder.".to_string(),
            )
        })?;

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
    /// A relevance score where higher is better. For vector search, this is the cosine similarity (1.0 is a perfect match). For keyword search, this is a placeholder 0.0.
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

/// Represents the data type of a field in a table schema.
/// This is a provider-agnostic representation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FieldType {
    String,
    Integer,
    Float,
    Boolean,
    Timestamp,
    Date,
    Bytes,
    Json,
}

/// Represents a single field (column) in a table schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableField {
    pub name: String,
    pub r#type: FieldType,
    pub description: Option<String>,
}

/// Represents the schema of a table in a provider-agnostic way.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TableSchema {
    pub fields: Vec<TableField>,
}

/// Represents the full set of options that can be received in an HTTP request
/// to the `/prompt` endpoint. It includes both library-level options and
/// server-specific fields like `db` and `model`.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct HttpRequestPromptOptions {
    // All fields from the library struct
    pub prompt: String,
    #[serde(default)]
    pub table_name: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub content_type: Option<ContentType>,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub instruction: Option<String>,
    #[serde(default)]
    pub answer_key: Option<String>,
    #[serde(default)]
    pub system_prompt_template: Option<String>,
    #[serde(default)]
    pub user_prompt_template: Option<String>,
    #[serde(default)]
    pub format_system_prompt_template: Option<String>,
    #[serde(default)]
    pub format_user_prompt_template: Option<String>,

    // Server-specific fields
    #[serde(default)]
    pub db: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

/// Converts the HTTP request options into the library's internal `ExecutePromptOptions`.
/// This is a crucial step in the decoupling process, as it separates the web-layer
/// data structure from the core logic's data structure, even though they are very similar.
impl From<HttpRequestPromptOptions> for ExecutePromptOptions {
    fn from(options: HttpRequestPromptOptions) -> Self {
        Self {
            prompt: options.prompt,
            table_name: options.table_name,
            project_id: options.project_id,
            content_type: options.content_type,
            context: options.context,
            instruction: options.instruction,
            answer_key: options.answer_key,
            system_prompt_template: options.system_prompt_template,
            user_prompt_template: options.user_prompt_template,
            format_system_prompt_template: options.format_system_prompt_template,
            format_user_prompt_template: options.format_user_prompt_template,
        }
    }
}

// --- Configuration Structs ---
// These structs define the shape of the application's configuration (`config.yml`)
// and are now centralized in the library crate to be shared with any consumer.

/// A fully resolved task configuration with non-optional fields.
/// This is derived from `TaskConfig` after configuration is loaded.
#[derive(Clone, Debug)]
pub struct ResolvedTask {
    pub provider: String,
    pub system_prompt: String,
    pub user_prompt: String,
}

/// Configuration for temporal reasoning.
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct TemporalReasoningConfig {
    #[serde(default = "default_temporal_keywords")]
    pub keywords: Vec<String>,
    #[serde(default = "default_temporal_property_name")]
    pub property_name: String,
}

/// Configuration for the embedding model provider.
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct EmbeddingConfig {
    pub api_url: String,
    pub model_name: String,
    pub api_key: Option<String>,
}

/// A reusable configuration for a specific AI provider instance.
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct ProviderConfig {
    /// The type of provider (e.g., "gemini", "local").
    pub provider: String,
    /// The API URL. Optional for providers like Gemini where it can be derived.
    pub api_url: Option<String>,
    /// The API key, which can be null for local providers.
    pub api_key: Option<String>,
    pub model_name: String,
}

/// Defines the prompts and provider for a specific application task from `config.yml`.
#[derive(Debug, Deserialize, Clone, Default)]
#[allow(dead_code)]
pub struct TaskConfig {
    /// The key of the provider to use from the `providers` map.
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub user_prompt: Option<String>,
}

fn default_temporal_keywords() -> Vec<String> {
    vec![
        "newest".to_string(),
        "latest".to_string(),
        "most recent".to_string(),
    ]
}

fn default_temporal_property_name() -> String {
    "release_date".to_string()
}

/// Provides a default value for the `port` field if not set in the environment.
fn default_port() -> u16 {
    9090
}

/// Provides a default value for the `db_url` field if not set in the environment.
fn default_db_url() -> String {
    constants::DEFAULT_DB_FILE.to_string()
}

/// Provides a default value for the `web_ingest_strategy` field.
fn default_web_ingest_strategy() -> String {
    "raw_html".to_string()
}

/// The root configuration structure, mapping directly to `config.yml`.
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct AppConfig {
    /// The port for the server to listen on. Loaded from `PORT` env var.
    #[serde(default = "default_port")]
    pub port: u16,
    /// The path to the SQLite database file. Loaded from `DB_URL` env var.
    #[serde(default = "default_db_url")]
    pub db_url: String,
    /// The directory for storing GitHub ingestion databases. Optional.
    #[serde(default)]
    pub github_db_dir: Option<String>,
    /// An optional API key for the Jina Reader service. Loaded from `JINA_API_KEY` env var.
    #[serde(default)]
    pub jina_api_key: Option<String>,
    /// The web ingestion strategy to use ("raw_html" or "jina"). Loaded from `WEB_INGEST_STRATEGY` env var.
    #[serde(default = "default_web_ingest_strategy")]
    pub web_ingest_strategy: String,

    /// Configuration for temporal reasoning.
    #[serde(default)]
    pub temporal_reasoning: Option<TemporalReasoningConfig>,

    /// Configuration for the text embedding model.
    pub embedding: EmbeddingConfig,
    /// A map of named, reusable AI provider configurations.
    pub providers: HashMap<String, ProviderConfig>,
    /// A map of tasks, each specifying a provider and prompts.
    pub tasks: HashMap<String, TaskConfig>,
}
