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
                write!(f, "Missing required environment variable: {key}")
            }
            ConfigError::Invalid(key, value) => {
                write!(f, "Invalid value for {key}: {value}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// Configuration for the server, loaded from environment variables.
#[derive(Debug)]
pub struct Config {
    pub ai_provider: String,
    pub ai_api_url: String,
    pub ai_api_key: Option<String>,
    pub ai_model: Option<String>,
    pub project_id: String,
    pub port: u16,
    // --- Prompt Templates ---
    pub query_system_prompt_template: Option<String>,
    pub query_user_prompt_template: Option<String>,
    pub format_system_prompt_template: Option<String>,
    pub format_user_prompt_template: Option<String>,
}

/// Loads configuration from environment variables.
///
/// Returns a `Result` containing the `Config` struct on success,
/// or a `ConfigError` if a required variable is missing or invalid.
pub fn get_config() -> Result<Config, ConfigError> {
    let ai_provider = env::var("AI_PROVIDER").unwrap_or_else(|_| "gemini".to_string());

    let ai_api_url =
        env::var("AI_API_URL").map_err(|_| ConfigError::Missing("AI_API_URL".to_string()))?;

    let ai_api_key = env::var("AI_API_KEY").ok();

    let ai_model = env::var("AI_MODEL").ok();

    let project_id = env::var("BIGQUERY_PROJECT_ID")
        .map_err(|_| ConfigError::Missing("BIGQUERY_PROJECT_ID".to_string()))?;

    let port = match env::var("PORT") {
        Ok(val) => val
            .parse::<u16>()
            .map_err(|_| ConfigError::Invalid("PORT".to_string(), val))?,
        Err(_) => 8080, // Default port
    };

    // --- Load Prompt Templates from Environment ---
    let query_system_prompt_template = env::var("QUERY_SYSTEM_PROMPT_TEMPLATE").ok();
    let query_user_prompt_template = env::var("QUERY_USER_PROMPT_TEMPLATE").ok();
    let format_system_prompt_template = env::var("FORMAT_SYSTEM_PROMPT_TEMPLATE").ok();
    let format_user_prompt_template = env::var("FORMAT_USER_PROMPT_TEMPLATE").ok();

    Ok(Config {
        ai_provider,
        ai_api_url,
        ai_api_key,
        ai_model,
        project_id,
        port,
        query_system_prompt_template,
        query_user_prompt_template,
        format_system_prompt_template,
        format_user_prompt_template,
    })
}
