//! # Prompt Override Logic Test
//!
//! This test file verifies the new, robust prompt override mechanism.
//! It ensures that prompts defined in `config.yml` correctly override the
//! hardcoded defaults from the `anyrag` library.

mod common;

use anyhow::Result;
use anyrag_server::{
    config::{get_config, AppConfig},
    state::build_app_state,
};
use common::TestApp;
use httpmock::{Method, MockServer};
use serde_json::json;
use std::fs::File;
use std::io::Write;
use tempfile::{tempdir, NamedTempFile};

#[tokio::test]
async fn test_yaml_overrides_default_prompts() -> Result<()> {
    // --- 1. Arrange ---
    let mock_server = MockServer::start();
    let db_file = NamedTempFile::new()?;
    let db_path = db_file.path();

    // --- 2. Create a custom config.yml with an override ---
    let temp_dir = tempdir()?;
    let config_path = temp_dir.path().join("config.yml");

    let override_system_prompt = "You are a pirate AI. Answer me question, matey!";
    let override_user_prompt =
        "Arr, what be the answer to this riddle: {prompt} from this here map: {context}";

    let yaml_content = format!(
        r#"
# This config uses the mock provider set up by TestApp.
db_url: "{}"
embedding:
  api_url: "{}"
  model_name: "mock-embedding-model"
providers:
  mock_provider:
    provider: "local"
    api_url: "{}"
    api_key: null
    model_name: "mock-chat-model"
# Override only the RAG synthesis task.
tasks:
  rag_synthesis:
    provider: "mock_provider"
    system_prompt: "{}"
    user_prompt: "{}"
"#,
        db_path.to_str().unwrap(),
        mock_server.url("/v1/embeddings"),
        mock_server.url("/v1/chat/completions"),
        override_system_prompt,
        override_user_prompt
    );

    let mut file = File::create(&config_path)?;
    file.write_all(yaml_content.as_bytes())?;

    // --- 3. Rebuild AppState with the new config ---
    let config: AppConfig = get_config(Some(config_path.to_str().unwrap()))?;
    let app_state_with_override = build_app_state(config).await?;

    // --- 4. Mock the AI response ---
    let rag_synthesis_mock = mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains("You are a pirate AI.")
            .body_contains("Arr, what be the answer to this riddle:");
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": "The treasure is buried under the old oak tree."}}]}),
        );
    });

    // --- 5. Act ---
    let task_config = app_state_with_override.tasks.get("rag_synthesis").unwrap();
    let ai_provider = app_state_with_override
        .ai_providers
        .get(&task_config.provider)
        .unwrap()
        .clone();
    let client = anyrag::PromptClientBuilder::new()
        .ai_provider(ai_provider)
        .storage_provider(Box::new(
            app_state_with_override.sqlite_provider.as_ref().clone(),
        ))
        .build()?;

    let options = anyrag::ExecutePromptOptions {
        prompt: "Where is the treasure?".to_string(),
        context: Some("The map shows an X.".to_string()),
        content_type: Some(anyrag::types::ContentType::Knowledge),
        system_prompt_template: Some(task_config.system_prompt.clone()),
        user_prompt_template: Some(task_config.user_prompt.clone()),
        ..Default::default()
    };

    let result = client.execute_prompt_with_options(options).await?;

    // --- 6. Assert ---
    assert_eq!(
        result.text,
        "The treasure is buried under the old oak tree."
    );
    rag_synthesis_mock.assert();

    Ok(())
}

#[tokio::test]
async fn test_default_prompts_are_used_when_no_override() -> Result<()> {
    // --- 1. Arrange ---
    // Spawn a TestApp. It creates an AppState with NO YAML overrides, so it will
    // use the hardcoded defaults from the library.
    let app = TestApp::spawn().await?;

    // --- 2. Mock the AI response for the default prompt ---
    let default_rag_system_prompt = "You are a strict, factual AI.";
    let rag_synthesis_mock = app.mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/chat/completions")
            .body_contains(default_rag_system_prompt);
        then.status(200).json_body(
            json!({"choices": [{"message": {"role": "assistant", "content": "The data indicates the value is 42."}}]}),
        );
    });

    // --- 3. Act ---
    // We get the AppState from the TestApp harness, which has the fully resolved
    // default prompts ready to use.
    let app_state = app.app_state.clone();

    let task_config = app_state.tasks.get("rag_synthesis").unwrap();
    let ai_provider = app_state
        .ai_providers
        .get(&task_config.provider)
        .unwrap()
        .clone();
    let client = anyrag::PromptClientBuilder::new()
        .ai_provider(ai_provider)
        .storage_provider(Box::new(app_state.sqlite_provider.as_ref().clone()))
        .build()?;

    let options = anyrag::ExecutePromptOptions {
        prompt: "What is the value?".to_string(),
        context: Some("The value is 42.".to_string()),
        content_type: Some(anyrag::types::ContentType::Knowledge),
        system_prompt_template: Some(task_config.system_prompt.clone()),
        user_prompt_template: Some(task_config.user_prompt.clone()),
        ..Default::default()
    };

    let result = client.execute_prompt_with_options(options).await?;

    // --- 4. Assert ---
    assert_eq!(result.text, "The data indicates the value is 42.");
    rag_synthesis_mock.assert();

    Ok(())
}
