//! # Application Configuration
//!
//! This module defines the configuration structure for the `anyrag-server` and
//! provides the logic for loading it from a `config.yml` file and environment
//! variables. This approach allows for a structured, flexible, and maintainable
//! configuration setup.

use anyrag::prompts::knowledge::KNOWLEDGE_RESTRUCTURING_SYSTEM_PROMPT;
use anyrag::prompts::tasks::*;
use config::{
    Config as ConfigBuilder, Environment, File, FileFormat, Value as ConfigValue,
    ValueKind as ConfigValueKind,
};
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use tracing::info;

/// A custom error type for configuration issues.
#[derive(Debug)]
pub enum ConfigError {
    /// Indicates an error from the underlying `config` crate.
    General(String),
    /// Indicates a required configuration file was not found.
    NotFound(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::General(msg) => write!(f, "Configuration error: {msg}"),
            ConfigError::NotFound(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<config::ConfigError> for ConfigError {
    fn from(err: config::ConfigError) -> Self {
        ConfigError::General(err.to_string())
    }
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

/// Provides a default value for the `port` field if not set in the environment.
fn default_port() -> u16 {
    9090
}
/// Provides a default value for the `db_url` field if not set in the environment.
fn default_db_url() -> String {
    "db/anyrag.db".to_string()
}

/// Provides a default value for the `web_ingest_strategy` field.
fn default_web_ingest_strategy() -> String {
    "raw_html".to_string()
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

/// Defines the prompts and provider for a specific application task.
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

/// Constructs a `config::Value` map of the default, hardcoded tasks from the library.
/// This serves as the base layer of configuration.
fn build_default_tasks() -> HashMap<String, ConfigValue> {
    let mut tasks = vec![
        (
            "query_generation",
            (
                "gemini_default",
                QUERY_GENERATION_SYSTEM_PROMPT,
                QUERY_GENERATION_USER_PROMPT,
            ),
        ),
        (
            "direct_generation",
            (
                "gemini_default",
                DIRECT_GENERATION_SYSTEM_PROMPT,
                DIRECT_GENERATION_USER_PROMPT,
            ),
        ),
        (
            "rag_synthesis",
            (
                "gemini_default",
                RAG_SYNTHESIS_SYSTEM_PROMPT,
                RAG_SYNTHESIS_USER_PROMPT,
            ),
        ),
        (
            "knowledge_distillation",
            (
                "gemini_default",
                KNOWLEDGE_RESTRUCTURING_SYSTEM_PROMPT, // Use the new YAML restructuring prompt.
                KNOWLEDGE_DISTILLATION_USER_PROMPT,
            ),
        ),
        (
            "query_analysis",
            (
                "gemini_default",
                QUERY_ANALYSIS_SYSTEM_PROMPT,
                QUERY_ANALYSIS_USER_PROMPT,
            ),
        ),
        (
            "llm_rerank",
            (
                "gemini_default",
                LLM_RERANK_SYSTEM_PROMPT,
                LLM_RERANK_USER_PROMPT,
            ),
        ),
        (
            "knowledge_augmentation",
            (
                "gemini_default",
                KNOWLEDGE_AUGMENTATION_SYSTEM_PROMPT,
                KNOWLEDGE_AUGMENTATION_USER_PROMPT,
            ),
        ),
        (
            "knowledge_metadata_extraction",
            (
                "gemini_default",
                KNOWLEDGE_METADATA_EXTRACTION_SYSTEM_PROMPT,
                KNOWLEDGE_METADATA_EXTRACTION_USER_PROMPT,
            ),
        ),
        (
            "context_agent",
            (
                "gemini_default",
                CONTEXT_AGENT_SYSTEM_PROMPT,
                CONTEXT_AGENT_USER_PROMPT,
            ),
        ),
        (
            "query_deconstruction",
            (
                "gemini_default",
                QUERY_DECONSTRUCTION_SYSTEM_PROMPT,
                QUERY_DECONSTRUCTION_USER_PROMPT,
            ),
        ),
    ];

    #[cfg(feature = "rss")]
    {
        tasks.push((
            "rss_summarization",
            (
                "gemini_default",
                RSS_SUMMARIZATION_SYSTEM_PROMPT,
                RSS_SUMMARIZATION_USER_PROMPT,
            ),
        ));
    }

    tasks
        .into_iter()
        .map(|(name, (provider, sys, user))| {
            let mut table = HashMap::new();
            table.insert("provider".to_string(), ConfigValue::from(provider));
            table.insert("system_prompt".to_string(), ConfigValue::from(sys));
            table.insert("user_prompt".to_string(), ConfigValue::from(user));
            (
                name.to_string(),
                ConfigValue::new(None, ConfigValueKind::Table(table)),
            )
        })
        .collect()
}

// Helper to read a file, substitute env vars, and return its content.
// Returns Ok(None) if the file does not exist, or an error if it fails to read.
fn read_and_substitute(path: &str) -> Result<Option<String>, ConfigError> {
    if !std::path::Path::new(path).exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)
        .map_err(|e| ConfigError::General(format!("Failed to read config file '{path}': {e}")))?;

    let re = Regex::new(r"\$\{(?P<var>[A-Z0-9_]+)\}").unwrap();
    let expanded_content = re.replace_all(&content, |caps: &regex::Captures| {
        let var_name = &caps["var"];
        env::var(var_name).unwrap_or_else(|_| "".to_string())
    });

    Ok(Some(expanded_content.to_string()))
}

/// Loads the application configuration from a file and environment variables.
///
/// This function reads the configuration from a file. It also merges in environment
/// variables, allowing for overrides and substitution in the YAML file.
/// - Top-level keys like `port` and `db_url` are overridden by `PORT` and `DB_URL`.
/// - Nested keys are overridden by `ANYRAG_...` variables (e.g., `ANYRAG_EMBEDDING__API_URL`).
pub fn get_config(config_path_override: Option<&str>) -> Result<AppConfig, ConfigError> {
    let base_path = env!("CARGO_MANIFEST_DIR");
    let mut builder = ConfigBuilder::builder()
        // Layer 1: Programmatic defaults from the library.
        .set_default("tasks", build_default_tasks())?;

    // Layer 2: Main Config (with Fallback)
    let main_config_path = if let Some(override_path) = config_path_override {
        override_path.to_string()
    } else {
        let user_config_path = format!("{base_path}/config.yml");
        if std::path::Path::new(&user_config_path).exists() {
            info!("Loading user-defined configuration from '{user_config_path}'.");
            user_config_path
        } else {
            let provider = env::var("AI_PROVIDER").unwrap_or_else(|_| "local".to_string());
            let fallback_path = format!("{base_path}/config.{provider}.yml");
            info!("'{user_config_path}' not found. Falling back to '{fallback_path}' based on AI_PROVIDER='{provider}'.");
            fallback_path
        }
    };

    let main_content = read_and_substitute(&main_config_path)?
        .ok_or_else(|| ConfigError::NotFound(format!("Main config file not found at '{main_config_path}'. Please ensure 'config.yml' exists or your AI_PROVIDER is set to load a valid template ('local' or 'gemini').")))?;
    builder = builder.add_source(File::from_str(&main_content, FileFormat::Yaml));

    // Layer 3: User Prompt Overrides (Optional)
    let user_prompt_path = format!("{base_path}/prompt.yml");
    if let Some(user_prompts_content) = read_and_substitute(&user_prompt_path)? {
        info!("Loading user prompt overrides from '{user_prompt_path}'.");
        builder = builder.add_source(File::from_str(&user_prompts_content, FileFormat::Yaml));
    }

    let settings = builder
        // Layer 4: Load environment variables for top-level keys like PORT.
        .add_source(Environment::default())
        // Layer 5: Load prefixed environment variables for deeper overrides.
        .add_source(
            Environment::with_prefix("ANYRAG")
                .prefix_separator("_")
                .try_parsing(true)
                .separator("__"),
        )
        .build()?;

    // Deserialize the fully resolved configuration into our `AppConfig` struct.
    let mut config: AppConfig = settings.try_deserialize()?;

    // After all layers, explicitly check for the JINA_API_KEY from the environment
    // if it hasn't been set by file substitution. This makes loading the key robust.
    if config.jina_api_key.is_none() {
        if let Ok(key) = env::var("JINA_API_KEY") {
            if !key.is_empty() {
                config.jina_api_key = Some(key);
            }
        }
    }

    Ok(config)
}
