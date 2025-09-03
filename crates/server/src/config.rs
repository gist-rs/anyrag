//! # Application Configuration
//!
//! This module defines the configuration structure for the `anyrag-server` and
//! provides the logic for loading it from a `config.yml` file and environment
//! variables. This approach allows for a structured, flexible, and maintainable
//! configuration setup.

use config::{Config as ConfigBuilder, Environment, File, FileFormat};
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;

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
            ConfigError::NotFound(path) => write!(f, "Configuration file not found: {path}. Please copy either `config.local.yml` or `config.gemini.yml` to `config.yml` to get started."),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<config::ConfigError> for ConfigError {
    fn from(err: config::ConfigError) -> Self {
        ConfigError::General(err.to_string())
    }
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

/// Configuration for the embedding model provider.
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct EmbeddingConfig {
    pub api_url: String,
    pub model_name: String,
}

/// A reusable configuration for a specific AI provider instance.
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct ProviderConfig {
    /// The type of provider (e.g., "gemini", "local").
    pub provider: String,
    pub api_url: String,
    /// The API key, which can be null for local providers.
    pub api_key: Option<String>,
    pub model_name: String,
}

/// Defines the prompts and provider for a specific application task.
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct TaskConfig {
    /// The key of the provider to use from the `providers` map.
    pub provider: String,
    pub system_prompt: String,
    pub user_prompt: String,
}

/// Loads the application configuration from a file and environment variables.
///
/// This function reads the configuration from a file. It also merges in environment
/// variables, allowing for overrides and substitution in the YAML file.
/// - Top-level keys like `port` and `db_url` are overridden by `PORT` and `DB_URL`.
/// - Nested keys are overridden by `ANYRAG_...` variables (e.g., `ANYRAG_EMBEDDING__API_URL`).
pub fn get_config(config_path_override: Option<&str>) -> Result<AppConfig, ConfigError> {
    let config_path = config_path_override.unwrap_or("config.yml");
    if !std::path::Path::new(config_path).exists() {
        return Err(ConfigError::NotFound(config_path.to_string()));
    }

    // Manually read the file and substitute env vars
    let content = fs::read_to_string(config_path)
        .map_err(|e| ConfigError::General(format!("Failed to read config file: {e}")))?;
    let re = Regex::new(r"\$\{(?P<var>[A-Z0-9_]+)\}").unwrap();
    let expanded_content = re.replace_all(&content, |caps: &regex::Captures| {
        let var_name = &caps["var"];
        env::var(var_name).unwrap_or_else(|_| "".to_string())
    });

    let settings = ConfigBuilder::builder()
        // 1. Load the YAML file after substituting env vars.
        .add_source(File::from_str(expanded_content.as_ref(), FileFormat::Yaml))
        // 2. Load unprefixed environment variables to override top-level keys like PORT.
        .add_source(Environment::default())
        // 3. Load prefixed environment variables for nested keys (less critical now, but good for direct overrides).
        .add_source(
            Environment::with_prefix("ANYRAG")
                .prefix_separator("_")
                .try_parsing(true)
                .separator("__"),
        )
        .build()?;

    // 4. Deserialize the fully resolved configuration into our `AppConfig` struct.
    settings.try_deserialize::<AppConfig>().map_err(Into::into)
}
