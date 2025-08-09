use anyrag::{
    providers::ai::{gemini::GeminiProvider, local::LocalAiProvider, AiProvider},
    ExecutePromptOptions, PromptClientBuilder,
};
use dotenvy::dotenv;
use std::env;
use std::sync::Once;

static INIT: Once = Once::new();

/// Initializes tracing and environment variables for tests.
fn setup() {
    INIT.call_once(|| {
        dotenv().ok();
        tracing_subscriber::fmt::init();
    });
}

/// This test verifies the override priority of prompt templates.
/// The expected priority is: API Request > Environment Variable > Default.
/// This test simulates the logic of the server's `prompt_handler` to
/// confirm that a prompt provided in an API request takes precedence
/// over one set via an environment variable.
#[tokio::test]
async fn test_prompt_override_priority() -> anyhow::Result<()> {
    setup();

    // --- 1. Build the AI Provider and Prompt Client ---
    let ai_provider_name = env::var("AI_PROVIDER").unwrap_or_else(|_| "gemini".to_string());
    let api_url = env::var("AI_API_URL").expect("AI_API_URL environment variable not set");
    let api_key = env::var("AI_API_KEY").ok();
    let ai_model = env::var("AI_MODEL").ok();
    let project_id =
        env::var("BIGQUERY_PROJECT_ID").expect("BIGQUERY_PROJECT_ID environment variable not set");

    let ai_provider = match ai_provider_name.as_str() {
        "gemini" => {
            let key = api_key.expect("AI_API_KEY is required for gemini provider");
            Box::new(GeminiProvider::new(api_url, key)?) as Box<dyn AiProvider>
        }
        "local" => {
            Box::new(LocalAiProvider::new(api_url, api_key, ai_model)?) as Box<dyn AiProvider>
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported AI provider: {ai_provider_name}"
            ))
        }
    };

    let client = PromptClientBuilder::new()
        .ai_provider(ai_provider)
        .bigquery_storage(project_id.clone())
        .await?
        .build()?;

    // --- 2. Define Prompts for Each Level of Override ---
    let env_prompt =
        "You are an AI that ONLY responds with the text '[ENV_PROMPT]' and nothing else."
            .to_string();
    let api_prompt =
        "You are an AI that ONLY responds with the text '[API_PROMPT]' and nothing else."
            .to_string();

    // --- 3. Test Case: API prompt should override the ENV prompt ---
    println!("--- Running Test Case 1: API overrides ENV ---");

    // This simulates an API request providing its own system prompt.
    let mut options_from_api = ExecutePromptOptions {
        prompt: "This prompt text does not matter for this test.".to_string(),
        system_prompt_template: Some(api_prompt),
        ..Default::default()
    };

    // This simulates the server's logic: if the API request doesn't have a prompt,
    // it would apply the one from the environment.
    if options_from_api.system_prompt_template.is_none() {
        options_from_api.system_prompt_template = Some(env_prompt.clone());
    }

    // Execute the prompt. The library will use the prompt from `options_from_api`.
    let result_api_override = client.execute_prompt_with_options(options_from_api).await?;
    println!("Response when API overrides ENV: '{result_api_override}'");

    // Assert that the API prompt was used.
    assert!(result_api_override.contains("[API_PROMPT]"));
    assert!(!result_api_override.contains("[ENV_PROMPT]"));

    // --- 4. Test Case: ENV prompt should be used when API prompt is absent ---
    println!("\n--- Running Test Case 2: ENV is used as fallback ---");

    // This simulates an API request that does NOT provide its own system prompt.
    let mut options_without_api_prompt = ExecutePromptOptions {
        prompt: "This prompt text does not matter for this test.".to_string(),
        system_prompt_template: None,
        ..Default::default()
    };

    // The server's logic applies the environment prompt because the API prompt is missing.
    if options_without_api_prompt.system_prompt_template.is_none() {
        options_without_api_prompt.system_prompt_template = Some(env_prompt);
    }

    let result_env_fallback = client
        .execute_prompt_with_options(options_without_api_prompt)
        .await?;
    println!("Response for ENV fallback: '{result_env_fallback}'");

    // Assert that the ENV prompt was used.
    assert!(result_env_fallback.contains("[ENV_PROMPT]"));
    assert!(!result_env_fallback.contains("[API_PROMPT]"));

    println!("\nOverride test completed successfully.");
    Ok(())
}
