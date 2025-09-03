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
    let mut builder = ConfigBuilder::builder();

    // Layer 1: Base Prompts (Required)
    let prompt_path = format!("{base_path}/config.prompt.yml");
    let prompts_content = read_and_substitute(&prompt_path)?
        .ok_or_else(|| ConfigError::NotFound(format!("Base prompt file not found at '{prompt_path}'. This file is a required part of the application.")))?;
    builder = builder.add_source(File::from_str(&prompts_content, FileFormat::Yaml));

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
    settings.try_deserialize::<AppConfig>().map_err(Into::into)
}
