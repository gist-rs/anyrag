//! # Configuration Tests
//!
//! This file contains tests for the configuration loading logic.
//! Since `anyrag-server` is a binary crate, we can't directly import
//! the `config` module. Instead, we include the source file directly
//! for testing purposes.

// Include the source code of the config module directly into the test binary.
// This is a workaround for testing code in a `main.rs` file from an integration test.
#[path = "../src/config.rs"]
mod config;

use self::config::{get_config, ConfigError};
use std::env;
use std::sync::Mutex;

// A mutex to ensure that tests modifying the environment run sequentially.
// This is crucial because environment variables are a shared, global resource,
// and running tests in parallel (`cargo test` default) could cause them to interfere.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// A helper function to clear all environment variables used by `get_config`.
/// This ensures a clean slate before each test runs.
fn clear_env_vars() {
    env::remove_var("AI_PROVIDER");
    env::remove_var("AI_API_URL");
    env::remove_var("AI_API_KEY");
    env::remove_var("AI_MODEL");
    env::remove_var("BIGQUERY_PROJECT_ID");
    env::remove_var("PORT");
    env::remove_var("QUERY_SYSTEM_PROMPT_TEMPLATE");
    env::remove_var("QUERY_USER_PROMPT_TEMPLATE");
    env::remove_var("FORMAT_SYSTEM_PROMPT_TEMPLATE");
    env::remove_var("FORMAT_USER_PROMPT_TEMPLATE");
}

#[test]
fn test_get_config_success_all_vars() {
    let _lock = ENV_LOCK.lock().unwrap();
    clear_env_vars();

    // Set all possible environment variables
    env::set_var("AI_PROVIDER", "local");
    env::set_var("AI_API_URL", "http://localhost:1234");
    env::set_var("BIGQUERY_PROJECT_ID", "test-project");
    env::set_var("AI_API_KEY", "test-api-key");
    env::set_var("AI_MODEL", "test-model");
    env::set_var("PORT", "9999");
    env::set_var("QUERY_SYSTEM_PROMPT_TEMPLATE", "qspt");
    env::set_var("QUERY_USER_PROMPT_TEMPLATE", "qupt");
    env::set_var("FORMAT_SYSTEM_PROMPT_TEMPLATE", "fspt");
    env::set_var("FORMAT_USER_PROMPT_TEMPLATE", "fupt");

    // Attempt to load the configuration
    let config = get_config().expect("Configuration should load successfully");

    // Assert that all values were parsed correctly
    assert_eq!(config.ai_provider, "local");
    assert_eq!(config.ai_api_url, "http://localhost:1234");
    assert_eq!(config.project_id, "test-project");
    assert_eq!(config.ai_api_key, Some("test-api-key".to_string()));
    assert_eq!(config.ai_model, Some("test-model".to_string()));
    assert_eq!(config.port, 9999);
    assert_eq!(
        config.query_system_prompt_template,
        Some("qspt".to_string())
    );
    assert_eq!(config.query_user_prompt_template, Some("qupt".to_string()));
    assert_eq!(
        config.format_system_prompt_template,
        Some("fspt".to_string())
    );
    assert_eq!(config.format_user_prompt_template, Some("fupt".to_string()));

    clear_env_vars();
}

#[test]
fn test_get_config_defaults() {
    let _lock = ENV_LOCK.lock().unwrap();
    clear_env_vars();

    // Set only the required variables
    env::set_var("AI_API_URL", "http://required.url");
    env::set_var("BIGQUERY_PROJECT_ID", "required-project");

    let config = get_config().expect("Configuration should load successfully");

    // Assert that default values are used for optional variables
    assert_eq!(config.ai_provider, "gemini");
    assert_eq!(config.port, 9090);
    assert!(config.ai_api_key.is_none());
    assert!(config.ai_model.is_none());
    assert!(config.query_system_prompt_template.is_none());

    clear_env_vars();
}

#[test]
fn test_get_config_missing_required_url() {
    let _lock = ENV_LOCK.lock().unwrap();
    clear_env_vars();

    // Miss the AI_API_URL
    env::set_var("BIGQUERY_PROJECT_ID", "test-project");

    let result = get_config();

    // Assert that the correct error is returned
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ConfigError::Missing(key) if key == "AI_API_URL"
    ));
    clear_env_vars();
}

#[test]
fn test_get_config_missing_required_project_id() {
    let _lock = ENV_LOCK.lock().unwrap();
    clear_env_vars();

    // Miss the BIGQUERY_PROJECT_ID
    env::set_var("AI_API_URL", "http://test.url");

    let result = get_config();

    // Assert that the correct error is returned
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ConfigError::Missing(key) if key == "BIGQUERY_PROJECT_ID"
    ));
    clear_env_vars();
}

#[test]
fn test_get_config_invalid_port() {
    let _lock = ENV_LOCK.lock().unwrap();
    clear_env_vars();

    // Set an invalid value for PORT
    env::set_var("AI_API_URL", "http://test.url");
    env::set_var("BIGQUERY_PROJECT_ID", "test-project");
    env::set_var("PORT", "not-a-number");

    let result = get_config();

    // Assert that the correct error is returned
    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), ConfigError::Invalid(key, val) if key == "PORT" && val == "not-a-number")
    );
    clear_env_vars();
}
