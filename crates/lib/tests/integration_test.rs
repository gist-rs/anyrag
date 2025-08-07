//! # Integration Tests
//!
//! This file contains integration tests for the `natural_language_to_sql` crate.
//!
//! **Note:** These tests require a valid Gemini API key and a BigQuery project with appropriate permissions.
//! You should set the `GEMINI_API_URL`, `GEMINI_API_KEY` and `BIGQUERY_PROJECT_ID` environment variables before running the tests.

use anyquery::{PromptClientBuilder, PromptError};
use dotenvy::dotenv;
use std::env;

/// Tests the successful execution of a valid prompt.
#[tokio::test]
async fn test_execute_prompt_success() {
    dotenv().ok();
    let gemini_url = env::var("GEMINI_API_URL").expect("GEMINI_API_URL not set");
    let gemini_api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set");
    let project_id = env::var("BIGQUERY_PROJECT_ID").expect("BIGQUERY_PROJECT_ID not set");

    let client = PromptClientBuilder::default()
        .gemini_url(gemini_url)
        .gemini_api_key(gemini_api_key)
        .bigquery_storage(project_id)
        .await
        .unwrap()
        .build()
        .unwrap();

    // This prompt assumes you have a public dataset like `bigquery-public-data.samples.shakespeare`
    let prompt = "Count the number of distinct corpus in the shakespeare dataset";
    let result = client
        .execute_prompt(
            prompt,
            Some("bigquery-public-data.samples.shakespeare"),
            None,
            None,
        )
        .await;

    if let Err(e) = &result {
        eprintln!("Error in test_execute_prompt_success: {e}");
    }
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(!output.is_empty());
}

/// Tests the handling of an invalid prompt that doesn't generate a valid SQL query.
#[tokio::test]
async fn test_execute_prompt_invalid_sql() {
    dotenv().ok();
    let gemini_url = env::var("GEMINI_API_URL").expect("GEMINI_API_URL not set");
    let gemini_api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set");
    let project_id = env::var("BIGQUERY_PROJECT_ID").expect("BIGQUERY_PROJECT_ID not set");

    let client = PromptClientBuilder::default()
        .gemini_url(gemini_url)
        .gemini_api_key(gemini_api_key)
        .bigquery_storage(project_id)
        .await
        .unwrap()
        .build()
        .unwrap();

    let prompt = "this is not a valid query";
    let result = client.execute_prompt(prompt, None, None, None).await;

    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        "The prompt did not result in a valid SQL query."
    );
}

/// Tests that the builder returns an error if the API key is missing.
#[tokio::test]
async fn test_builder_missing_api_key() {
    dotenv().ok();
    let project_id = env::var("BIGQUERY_PROJECT_ID").expect("BIGQUERY_PROJECT_ID not set");

    let builder_result = PromptClientBuilder::default()
        .bigquery_storage(project_id)
        .await
        .unwrap()
        .build();

    assert!(matches!(
        builder_result.unwrap_err(),
        PromptError::MissingApiKey
    ));
}

/// Tests that the builder returns an error if the storage provider is missing.
#[tokio::test]
async fn test_builder_missing_storage_provider() {
    dotenv().ok();
    let gemini_url = env::var("GEMINI_API_URL").expect("GEMINI_API_URL not set");
    let gemini_api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set");

    let builder_result = PromptClientBuilder::default()
        .gemini_url(gemini_url)
        .gemini_api_key(gemini_api_key)
        .build();

    assert!(matches!(
        builder_result.unwrap_err(),
        PromptError::MissingStorageProvider
    ));
}

/// Tests the successful execution of a valid prompt with a formatting instruction.
#[tokio::test]
async fn test_execute_prompt_with_formatting() {
    dotenv().ok();
    let gemini_url = env::var("GEMINI_API_URL").expect("GEMINI_API_URL not set");
    let gemini_api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set");
    let project_id = env::var("BIGQUERY_PROJECT_ID").expect("BIGQUERY_PROJECT_ID not set");

    let client = PromptClientBuilder::default()
        .gemini_url(gemini_url)
        .gemini_api_key(gemini_api_key)
        .bigquery_storage(project_id)
        .await
        .unwrap()
        .build()
        .unwrap();

    let prompt = "What is the total word_count for the corpus 'kinghenryv'?";
    let table_name = "bigquery-public-data.samples.shakespeare";
    let instruction = "Answer with only the number with thousand format.";
    let result = client
        .execute_prompt(prompt, Some(table_name), Some(instruction), None)
        .await;

    if let Err(e) = &result {
        eprintln!("Error in test_execute_prompt_with_formatting: {e}");
    }
    assert!(result.is_ok());
    let output = result.unwrap();

    println!("{output}");

    assert!(!output.contains("f0_")); // Should not contain the raw JSON key
    assert!(output.contains("27,894"));
}
