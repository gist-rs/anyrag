use std::env;

/// A custom error type for configuration issues.
#[derive(Debug)]
pub enum ConfigError {
    /// Indicates a required environment variable is missing.
    Missing(String),
    /// Indicates a variable could not be parsed into its target type.
    Invalid(String, String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Missing(key) => {
                write!(f, "Missing required environment variable: {}", key)
            }
            ConfigError::Invalid(key, value) => {
                write!(f, "Invalid value for {}: {}", key, value)
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// Configuration for the BigQuery Tools server.
///
/// This configuration is loaded from environment variables.
#[derive(Debug)]
pub struct Config {
    pub gemini_api_url: String,
    pub gemini_api_key: String,
    pub project_id: String,
    pub port: u16,
}

/// Loads configuration from environment variables.
///
/// Returns a `Result` containing the `Config` struct on success,
/// or a `ConfigError` if a required variable is missing or invalid.
pub fn get_config() -> Result<Config, ConfigError> {
    let gemini_api_key = env::var("GEMINI_API_KEY")
        .map_err(|_| ConfigError::Missing("GEMINI_API_KEY".to_string()))?;

    let project_id = env::var("BIGQUERY_PROJECT_ID")
        .map_err(|_| ConfigError::Missing("BIGQUERY_PROJECT_ID".to_string()))?;

    let gemini_api_url = env::var("GEMINI_API_URL")
        .unwrap_or_else(|_| "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent".to_string());

    let port = match env::var("PORT") {
        Ok(val) => val
            .parse::<u16>()
            .map_err(|_| ConfigError::Invalid("PORT".to_string(), val))?,
        Err(_) => 8080, // Default port
    };

    Ok(Config {
        gemini_api_url,
        gemini_api_key,
        project_id,
        port,
    })
}
