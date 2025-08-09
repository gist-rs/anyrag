// This declaration makes the `common` module available to the tests in this file.
mod common;

use crate::common::{MockAiProvider, MockStorageProvider};
use anyrag::{providers::ai::AiProvider, ExecutePromptOptions, PromptClient, PromptClientBuilder};
use std::sync::Arc;

// --- Test Setup ---
// This section defines a simplified "app state" similar to the one in `anyrag-server`,
// which allows for testing the logic of how server-wide default prompts are applied
// and overridden by API requests.

#[derive(Clone)]
struct TestAppState {
    prompt_client: Arc<PromptClient>,
    query_system_prompt_template: Option<String>,
    query_user_prompt_template: Option<String>,
    format_system_prompt_template: Option<String>,
    format_user_prompt_template: Option<String>,
}

fn setup_mock_app_state(
    mock_provider: Box<dyn AiProvider>,
    env_templates: (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ),
) -> TestAppState {
    let (
        query_system_prompt_template,
        query_user_prompt_template,
        format_system_prompt_template,
        format_user_prompt_template,
    ) = env_templates;

    let client = PromptClientBuilder::new()
        .ai_provider(mock_provider)
        .storage_provider(Box::new(MockStorageProvider)) // Use the shared mock
        .build()
        .unwrap();

    TestAppState {
        prompt_client: Arc::new(client),
        query_system_prompt_template,
        query_user_prompt_template,
        format_system_prompt_template,
        format_user_prompt_template,
    }
}

/// Simulates the logic in the main server handler where environment-variable-based
/// default prompts are applied if the incoming request doesn't provide its own.
fn apply_server_defaults(
    mut options: ExecutePromptOptions,
    state: &TestAppState,
) -> ExecutePromptOptions {
    if options.system_prompt_template.is_none() {
        options.system_prompt_template = state.query_system_prompt_template.clone();
    }
    if options.user_prompt_template.is_none() {
        options.user_prompt_template = state.query_user_prompt_template.clone();
    }
    if options.format_system_prompt_template.is_none() {
        options.format_system_prompt_template = state.format_system_prompt_template.clone();
    }
    if options.format_user_prompt_template.is_none() {
        options.format_user_prompt_template = state.format_user_prompt_template.clone();
    }
    options
}

// --- Main Test Suite ---

/// This test comprehensively verifies the prompt override logic.
/// It ensures that prompts provided in an API request take precedence over
/// server-wide defaults (simulated as environment variables), and that those
/// server-wide defaults take precedence over the hardcoded defaults in the library.
#[tokio::test]
async fn test_full_prompt_override_logic() {
    println!("--- Testing Full Prompt Override Logic ---");

    #[allow(clippy::complexity)]
    async fn run_test_stage(
        stage_name: &str,
        env_templates: (
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
        api_options: ExecutePromptOptions,
        env_fallback_options: ExecutePromptOptions,
        expected_api_string: &str,
        expected_env_string: &str,
        prompt_index: usize, // 0 for system, 1 for user
        call_index: usize,   // 0 for query, 1 for format
    ) {
        println!("\n--- Stage: {stage_name} ---");

        // --- Test API > ENV ---
        // Verifies that a prompt template passed in the API request body overrides
        // any template set in the server's environment.
        let api_responses = vec!["SELECT 1".to_string(), "Ok".to_string()];
        let mock_provider_api = MockAiProvider::new(api_responses);
        let history_api = mock_provider_api.call_history.clone();
        let env_state_api =
            setup_mock_app_state(Box::new(mock_provider_api), env_templates.clone());

        let final_api_options = apply_server_defaults(api_options.clone(), &env_state_api);
        let _ = env_state_api
            .prompt_client
            .execute_prompt_with_options(final_api_options)
            .await;

        let last_call_api = {
            let guard = history_api.read().unwrap();
            guard.get(call_index).unwrap().clone()
        };

        let prompt_to_check_api = if prompt_index == 0 {
            &last_call_api.0
        } else {
            &last_call_api.1
        };
        assert!(prompt_to_check_api.contains(expected_api_string));
        println!("  - API > ENV: PASSED");

        // --- Test ENV > Default ---
        // Verifies that if no template is in the API request, the server's
        // environment template is used instead of the library's default.
        let env_responses = vec!["SELECT 1".to_string(), "Ok".to_string()];
        let mock_provider_env = MockAiProvider::new(env_responses);
        let history_env = mock_provider_env.call_history.clone();
        let env_state_env = setup_mock_app_state(Box::new(mock_provider_env), env_templates);

        let final_env_options = apply_server_defaults(env_fallback_options.clone(), &env_state_env);
        let _ = env_state_env
            .prompt_client
            .execute_prompt_with_options(final_env_options)
            .await;

        let last_call_env = {
            let guard = history_env.read().unwrap();
            guard.get(call_index).unwrap().clone()
        };

        let prompt_to_check_env = if prompt_index == 0 {
            &last_call_env.0
        } else {
            &last_call_env.1
        };
        assert!(prompt_to_check_env.contains(expected_env_string));
        println!("  - ENV > Default: PASSED");
    }

    let base_query_options = ExecutePromptOptions {
        prompt: "p".to_string(),
        table_name: Some("t".to_string()),
        ..Default::default()
    };
    let base_format_options = ExecutePromptOptions {
        instruction: Some("i".to_string()),
        ..base_query_options.clone()
    };

    run_test_stage(
        "Query System Prompt",
        (Some("[ENV_QUERY_SYSTEM]".to_string()), None, None, None),
        ExecutePromptOptions {
            system_prompt_template: Some("[API_QUERY_SYSTEM]".to_string()),
            ..base_query_options.clone()
        },
        base_query_options.clone(),
        "[API_QUERY_SYSTEM]",
        "[ENV_QUERY_SYSTEM]",
        0, // Check system prompt
        0, // Check first AI call (query generation)
    )
    .await;

    run_test_stage(
        "Query User Prompt",
        (None, Some("[ENV_QUERY_USER]".to_string()), None, None),
        ExecutePromptOptions {
            user_prompt_template: Some("[API_QUERY_USER]".to_string()),
            ..base_query_options.clone()
        },
        base_query_options.clone(),
        "[API_QUERY_USER]",
        "[ENV_QUERY_USER]",
        1, // Check user prompt
        0, // Check first AI call (query generation)
    )
    .await;

    run_test_stage(
        "Format System Prompt",
        (None, None, Some("[ENV_FORMAT_SYSTEM]".to_string()), None),
        ExecutePromptOptions {
            format_system_prompt_template: Some("[API_FORMAT_SYSTEM]".to_string()),
            ..base_format_options.clone()
        },
        base_format_options.clone(),
        "[API_FORMAT_SYSTEM]",
        "[ENV_FORMAT_SYSTEM]",
        0, // Check system prompt
        1, // Check second AI call (formatting)
    )
    .await;

    run_test_stage(
        "Format User Prompt",
        (None, None, None, Some("[ENV_FORMAT_USER]".to_string())),
        ExecutePromptOptions {
            format_user_prompt_template: Some("[API_FORMAT_USER]".to_string()),
            ..base_format_options.clone()
        },
        base_format_options.clone(),
        "[API_FORMAT_USER]",
        "[ENV_FORMAT_USER]",
        1, // Check user prompt
        1, // Check second AI call (formatting)
    )
    .await;

    println!("\nComprehensive override test finished successfully.");
}
