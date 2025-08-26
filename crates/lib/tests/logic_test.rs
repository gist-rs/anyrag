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
        final_result.text, today_str,
        "The final result should be the direct answer from the AI"
    );
    assert!(
        final_result.generated_sql.is_none(),
        "No SQL should be generated for a direct answer"
    );
    assert!(
        final_result.database_result.is_none(),
        "No database result should exist for a direct answer"
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

    // --- Validate RFC2822 date ---
    let rfc2822_line = lines
        .get(today_header_index + 1)
        .unwrap_or_else(|| panic!("Did not find an RFC2822 date line after the '# TODAY' header"));
    assert!(
        rfc2822_line.starts_with("RFC2822: "),
        "RFC2822 line should have the correct prefix"
    );
    let rfc2822_date_str = rfc2822_line.trim_start_matches("RFC2822: ").trim();
    let parsed_rfc2822_date = DateTime::parse_from_rfc2822(rfc2822_date_str);
    assert!(
        parsed_rfc2822_date.is_ok(),
        "The date string '{rfc2822_date_str}' should be a valid RFC 2822 datetime"
    );

    // --- Validate UTC (RFC3339) date ---
    let utc_line = lines
        .get(today_header_index + 2)
        .unwrap_or_else(|| panic!("Did not find a UTC date line after the RFC2822 line"));
    assert!(
        utc_line.starts_with("UTC: "),
        "UTC line should have the correct prefix"
    );
    let utc_date_str = utc_line.trim_start_matches("UTC: ").trim();
    let parsed_utc_date = DateTime::parse_from_rfc3339(utc_date_str);
    assert!(
        parsed_utc_date.is_ok(),
        "The date string '{utc_date_str}' should be a valid RFC 3339 (ISO8601) datetime"
    );
}
