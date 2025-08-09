//! # Logic Tests
//!
//! This file contains tests for the core logic of the `anyrag` library,
//! focusing on prompt construction and handling different types of AI responses.

mod common;

use crate::common::{setup_tracing, MockAiProvider, MockStorageProvider};
use anyrag::{ExecutePromptOptions, PromptClientBuilder};
use chrono::DateTime;

/// Tests that:
/// 1. The current date/time is correctly injected into the prompt context for the AI.
/// 2. The system correctly handles and returns a non-query, direct answer from the AI,
///    which is crucial for time-based questions.
#[tokio::test]
async fn test_today_in_context_and_direct_answer_handling() {
    setup_tracing();

    // 1. Setup mock AI provider to return a direct answer
    let today_str = "Today is Saturday, August 9, 2025.";
    let mock_ai_provider = MockAiProvider::new(vec![today_str.to_string()]);
    let call_history = mock_ai_provider.call_history.clone();

    // 2. Setup client with mock providers
    let client = PromptClientBuilder::new()
        .ai_provider(Box::new(mock_ai_provider))
        .storage_provider(Box::new(MockStorageProvider))
        .build()
        .unwrap();

    // 3. Define options for the prompt
    let options = ExecutePromptOptions {
        prompt: "What's today?".to_string(),
        ..Default::default()
    };

    // 4. Execute the full prompt flow
    let final_result = client
        .execute_prompt_with_options(options)
        .await
        .expect("Execution should not fail");

    // 5. Assert that the final result is the direct answer from the AI
    assert_eq!(
        final_result, today_str,
        "The final result should be the direct answer from the AI"
    );

    // 6. Assert that the prompt context sent TO the AI was correctly formatted
    let history = call_history.read().unwrap();
    assert_eq!(
        history.len(),
        1,
        "Expected exactly one call to the AI provider"
    );

    let (_system_prompt, user_prompt) = &history[0];

    // Check if the user prompt contains the # TODAY section
    assert!(
        user_prompt.contains("# TODAY\n"),
        "User prompt should contain the '# TODAY' section"
    );

    // Extract and validate the date from the context
    let lines: Vec<&str> = user_prompt.lines().collect();
    let today_header_index = lines
        .iter()
        .position(|&line| line.trim() == "# TODAY")
        .expect("Could not find '# TODAY' header in the user prompt");

    let date_line = lines
        .get(today_header_index + 1)
        .unwrap_or_else(|| panic!("Did not find a date line after the '# TODAY' header"));

    // The format is RFC 2822, so we try to parse it.
    // The format is RFC 2822, so we try to parse it to confirm its validity.
    let parsed_date = DateTime::parse_from_rfc2822(date_line.trim());
    assert!(
        parsed_date.is_ok(),
        "The date string '{}' in the context should be a valid RFC 2822 datetime",
        date_line
    );
}
