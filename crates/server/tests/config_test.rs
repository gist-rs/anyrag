//! # Configuration Tests
//!
//! This file contains tests for the new `config.yml` loading logic.
//! Each test is self-contained to prevent interference from shared state,
//! especially environment variables.

#[path = "../src/config.rs"]
mod config;

use self::config::{get_config, ConfigError};
use serial_test::serial;
use std::env;
use std::fs::File;
use std::io::Write;
use tempfile::{tempdir, TempDir};
/// A test fixture to manage isolated configuration files for each test.
/// It creates a temporary directory and a path for a config file within it.
/// The directory and its contents are automatically cleaned up when the fixture is dropped.
struct TestFixture {
    _temp_dir: TempDir,
    config_path: String,
}

impl TestFixture {
    fn new() -> Self {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("config.yml");
        Self {
            _temp_dir: temp_dir,
            config_path: config_path.to_str().unwrap().to_string(),
        }
    }

    /// Creates a `config.yml` file with the given content in the temporary directory.
    fn create_config_file(&self, content: &str) {
        let mut file = File::create(&self.config_path).expect("Failed to create test config.yml");
        file.write_all(content.as_bytes())
            .expect("Failed to write to test config.yml");
    }
}

/// A simple RAII guard to unset an environment variable when it goes out of scope.
/// This ensures that environment changes are cleaned up even if a test panics.
struct EnvVarGuard {
    key: String,
    old_value: Option<String>,
}

impl EnvVarGuard {
    fn new(key: &str, value: &str) -> Self {
        let old_value = env::var(key).ok();
        println!("[EnvVarGuard] Setting '{key}' to '{value}'. Old value was '{old_value:?}'.");
        env::set_var(key, value);
        Self {
            key: key.to_string(),
            old_value,
        }
    }

    /// Creates a guard that clears an environment variable for its scope.
    fn clear(key: &str) -> Self {
        let old_value = env::var(key).ok();
        if old_value.is_some() {
            println!("[EnvVarGuard] Clearing '{key}'. Old value was '{old_value:?}'.");
            env::remove_var(key);
        } else {
            println!("[EnvVarGuard] Clearing '{key}'. It was not set.");
        }
        Self {
            key: key.to_string(),
            old_value,
        }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(val) = &self.old_value {
            println!("[EnvVarGuard] Restoring '{}' to '{}'.", self.key, val);
            env::set_var(&self.key, val);
        } else {
            println!("[EnvVarGuard] Unsetting '{}'.", self.key);
            env::remove_var(&self.key);
        }
    }
}

#[test]
#[serial]
fn test_successful_config_load() {
    // Clear any potentially conflicting environment variables to ensure a clean test.
    let _guards = [
        EnvVarGuard::clear("PORT"),
        EnvVarGuard::clear("DB_URL"),
        EnvVarGuard::clear("ANYRAG_EMBEDDING__API_URL"),
        EnvVarGuard::clear("ANYRAG_PROVIDERS__TEST_PROVIDER__API_URL"),
        EnvVarGuard::clear("ANYRAG_PROVIDERS__TEST_PROVIDER__API_KEY"),
    ];
    let fixture = TestFixture::new();
    let yaml_content = r#"
port: 8080
db_url: "db/anyrag.db"
embedding:
  api_url: "http://localhost:11434/v1/embeddings"
  model_name: "test-embedding-model"
providers:
  test_provider:
    provider: "local"
    api_url: "http://localhost:11434/v1/chat/completions"
    api_key: "my_secret_key"
    model_name: "test-chat-model"
tasks:
  test_task:
    provider: "test_provider"
    system_prompt: "You are a test assistant."
    user_prompt: "Hello, {prompt}."
"#;
    fixture.create_config_file(yaml_content);

    let config =
        get_config(Some(&fixture.config_path)).expect("Configuration should load successfully");

    assert_eq!(config.port, 8080);
    assert_eq!(config.db_url, "db/anyrag.db");
    let provider = config.providers.get("test_provider").unwrap();
    assert_eq!(provider.api_key, Some("my_secret_key".to_string()));
}

#[test]
#[serial]
fn test_env_var_override_and_defaults() {
    let fixture = TestFixture::new();
    let yaml_content = r#"
# port and db_url are omitted to test defaults and env var overrides.
embedding:
  api_url: "${ANYRAG_EMBEDDING__API_URL}"
  model_name: "default-model"
providers:
  gemini:
    provider: "gemini"
    api_url: "${ANYRAG_PROVIDERS__GEMINI__API_URL}"
    api_key: "${ANYRAG_PROVIDERS__GEMINI__API_KEY}"
    model_name: "gemini-2.5-flash-lite"
tasks:
  default: { provider: "gemini", system_prompt: "sys", user_prompt: "user" }
"#;
    fixture.create_config_file(yaml_content);

    // Set environment variables. They will be cleaned up automatically when the guards are dropped.
    let _port_guard = EnvVarGuard::new("PORT", "9999");
    let _db_url_guard = EnvVarGuard::new("DB_URL", "env.db");
    let _embed_url_guard = EnvVarGuard::new("ANYRAG_EMBEDDING__API_URL", "http://env-embed.com");
    let _ai_url_guard = EnvVarGuard::new("ANYRAG_PROVIDERS__GEMINI__API_URL", "http://env-ai.com");
    let _api_key_guard = EnvVarGuard::new("ANYRAG_PROVIDERS__GEMINI__API_KEY", "env-secret-key");

    let config = get_config(Some(&fixture.config_path))
        .expect("Configuration should load with env substitution");

    assert_eq!(config.port, 9999, "PORT should be overridden by env var");
    assert_eq!(
        config.db_url, "env.db",
        "DB_URL should be overridden by env var"
    );
    assert_eq!(
        config.embedding.api_url, "http://env-embed.com",
        "Nested embedding.api_url should be substituted"
    );
    let provider = config.providers.get("gemini").unwrap();
    assert_eq!(
        provider.api_url, "http://env-ai.com",
        "Provider API URL should be overridden"
    );
    assert_eq!(
        provider.api_key,
        Some("env-secret-key".to_string()),
        "Provider API key should be overridden"
    );
}

#[test]
#[serial]
fn test_config_file_not_found() {
    // Don't create the file, just use the path from the fixture.
    let fixture = TestFixture::new();
    let result = get_config(Some(&fixture.config_path));
    assert!(
        matches!(result, Err(ConfigError::NotFound(_))),
        "Expected NotFound error, but got {result:?}"
    );
}

#[test]
#[serial]
fn test_config_parsing_error() {
    let fixture = TestFixture::new();
    // Invalid YAML: `providers` is an array, but should be a map.
    let invalid_yaml =
        "embedding: { api_url: 'url', model_name: 'model' }\nproviders:\n  - item1\ntasks: {}";
    fixture.create_config_file(invalid_yaml);
    let result = get_config(Some(&fixture.config_path));
    assert!(
        matches!(result, Err(ConfigError::General(_))),
        "Expected General error for parsing, but got {result:?}"
    );
}

#[test]
#[serial]
fn test_missing_required_field_error() {
    // Clear env vars that might cause the config to pass validation unexpectedly.
    let _guards = [EnvVarGuard::clear("PORT"), EnvVarGuard::clear("DB_URL")];
    let fixture = TestFixture::new();
    // This YAML is missing the required `model_name` field inside `embedding`.
    let incomplete_yaml = r#"
port: 8000
db_url: "db/anyrag.db"
embedding:
  api_url: "some-url"
providers: {}
tasks: {}
"#;
    fixture.create_config_file(incomplete_yaml);
    let result = get_config(Some(&fixture.config_path));

    match result {
        Err(ConfigError::General(msg)) => {
            assert!(
                msg.contains("missing field `model_name`"),
                "Error message did not indicate the correct missing field. Got: {msg}"
            );
        }
        other => {
            panic!(
                "Expected a General config error for missing field, but got {other:?}. This might indicate the test is succeeding when it should fail."
            );
        }
    }
}
