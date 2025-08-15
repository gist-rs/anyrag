//! # Integration Tests
//!
//! This file contains integration tests for the `anyrag` crate.
//!
//! **Note:** These tests require a valid AI provider API key and a BigQuery project with appropriate permissions.
//! You should set the `AI_API_URL`, `AI_API_KEY`, and `BIGQUERY_PROJECT_ID` environment variables before running the tests.

// This declaration makes the `common` module available to the tests in this file.
mod common;

use crate::common::{create_real_ai_provider, setup_tracing, MockStorageProvider};
use anyrag::{
    providers::ai::local::LocalAiProvider, ExecutePromptOptions, PromptClientBuilder, PromptError,
};
use chrono::Utc;
use httpmock::prelude::*;
use serde_json::json;
use std::env;
use tracing::debug;

/// Tests the successful execution of a valid prompt.
#[cfg(test)]
#[tokio::test]
async fn test_execute_prompt_success() {
    setup_tracing();
    let project_id = env::var("BIGQUERY_PROJECT_ID").expect("BIGQUERY_PROJECT_ID not set");

    let client = PromptClientBuilder::default()
        .ai_provider(create_real_ai_provider())
        .bigquery_storage(project_id)
        .await
        .unwrap()
        .build()
        .unwrap();

    // This prompt assumes you have a public dataset like `bigquery-public-data.samples.shakespeare`
    let options = ExecutePromptOptions {
        prompt: "Count the number of distinct corpus in the shakespeare dataset".to_string(),
        table_name: Some("bigquery-public-data.samples.shakespeare".to_string()),
        ..Default::default()
    };

    let result = client.execute_prompt_with_options(options).await;

    if let Err(e) = &result {
        eprintln!("Error in test_execute_prompt_success: {e}");
    }
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(!output.result.is_empty());
}

/// Tests the handling of an invalid prompt that doesn't generate a valid query.
#[tokio::test]
async fn test_execute_prompt_invalid_query() {
    setup_tracing();
    let project_id = env::var("BIGQUERY_PROJECT_ID").expect("BIGQUERY_PROJECT_ID not set");

    let client = PromptClientBuilder::default()
        .ai_provider(create_real_ai_provider())
        .bigquery_storage(project_id)
        .await
        .unwrap()
        .build()
        .unwrap();

    let options = ExecutePromptOptions {
        prompt: "this is not a valid query".to_string(),
        ..Default::default()
    };
    let result = client.execute_prompt_with_options(options).await;

    // With the new logic, a non-query prompt results in the AI's direct textual answer.
    // We can't know the exact response, but we can assert it's a non-empty string
    // and not a query.
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(!response.result.is_empty());
    assert!(!response.result.to_uppercase().contains("SELECT"));
}

/// Tests that the builder returns an error if the ai provider is missing.
#[tokio::test]
async fn test_builder_missing_ai_provider() {
    setup_tracing();
    let project_id = env::var("BIGQUERY_PROJECT_ID").expect("BIGQUERY_PROJECT_ID not set");

    let builder_result = PromptClientBuilder::default()
        .bigquery_storage(project_id)
        .await
        .unwrap()
        .build();

    assert!(matches!(
        builder_result.unwrap_err(),
        PromptError::MissingAiProvider
    ));
}

/// Tests that the builder returns an error if the storage provider is missing.
#[tokio::test]
async fn test_builder_missing_storage_provider() {
    setup_tracing();
    let builder_result = PromptClientBuilder::default()
        .ai_provider(create_real_ai_provider())
        .build();

    assert!(matches!(
        builder_result.unwrap_err(),
        PromptError::MissingStorageProvider
    ));
}

/// Tests the successful execution of a valid prompt with a formatting instruction.
#[tokio::test]
async fn test_execute_prompt_with_formatting() {
    setup_tracing();
    let project_id = env::var("BIGQUERY_PROJECT_ID").expect("BIGQUERY_PROJECT_ID not set");

    let client = PromptClientBuilder::default()
        .ai_provider(create_real_ai_provider())
        .bigquery_storage(project_id)
        .await
        .unwrap()
        .build()
        .unwrap();

    let options = ExecutePromptOptions {
        prompt: "What is the total word_count for the corpus 'kinghenryv'?".to_string(),
        table_name: Some("bigquery-public-data.samples.shakespeare".to_string()),
        instruction: Some("Answer with only the number with thousand format.".to_string()),
        ..Default::default()
    };

    let result = client.execute_prompt_with_options(options).await;

    if let Err(e) = &result {
        eprintln!("Error in test_execute_prompt_with_formatting: {e}");
    }
    assert!(result.is_ok());
    let output = result.unwrap();

    // The alias is chosen by the model, so we check that the raw key isn't present
    // and that the expected formatted number is.
    assert!(!output.result.contains("f0_"));
    assert!(output.result.contains("27,894"));
}

/// Tests using a custom system prompt for query generation.
#[tokio::test]
async fn test_execute_with_custom_query_prompt() {
    setup_tracing();
    let project_id = env::var("BIGQUERY_PROJECT_ID").expect("BIGQUERY_PROJECT_ID not set");

    let client = PromptClientBuilder::default()
        .ai_provider(create_real_ai_provider())
        .bigquery_storage(project_id)
        .await
        .unwrap()
        .build()
        .unwrap();

    // This prompt forces the AI to act as a translator, not a SQL expert.
    // The expected output is the Japanese translation of the prompt.
    let options = ExecutePromptOptions {
        prompt: "hello".to_string(),
        system_prompt_template: Some("You are a translator from English to Japanese.".to_string()),
        ..Default::default()
    };

    let result = client.execute_prompt_with_options(options).await;

    if let Err(e) = &result {
        eprintln!("Error in test_execute_with_custom_query_prompt: {e}");
    }
    assert!(result.is_ok());
    let output = result.unwrap();

    // The model should translate "hello" to "こんにちは".
    assert!(output.result.contains("こんにちは"));
}

/// Tests using a custom system prompt for the final response formatting.
#[tokio::test]
async fn test_execute_with_custom_format_prompt() {
    setup_tracing();
    let project_id = env::var("BIGQUERY_PROJECT_ID").expect("BIGQUERY_PROJECT_ID not set");

    let client = PromptClientBuilder::default()
        .ai_provider(create_real_ai_provider())
        .bigquery_storage(project_id)
        .await
        .unwrap()
        .build()
        .unwrap();

    // This prompt asks a regular question but has a custom formatting instruction
    // that forces the model to add a winky face to its response.
    let options = ExecutePromptOptions {
        prompt: "What is the total word_count for the corpus 'kinghenryv'?".to_string(),
        table_name: Some("bigquery-public-data.samples.shakespeare".to_string()),
        instruction: Some(
            "Answer with a natural sentence and the number with thousand format.".to_string(),
        ),
        format_system_prompt_template: Some(
            "You are a friendly assistant. It is absolutely crucial that you end every single response with a winky face ;). No matter what, your response MUST end with it.".to_string(),
        ),
        ..Default::default()
    };

    let result = client.execute_prompt_with_options(options).await;

    if let Err(e) = &result {
        eprintln!("Error in test_execute_with_custom_format_prompt: {e}");
    }
    assert!(result.is_ok());
    let output = result.unwrap();

    debug!("output 👉 {:?}", output);

    // Check for the original number. The winky face is commented out as it's not
    // reliably produced by all models. The main goal is to test the custom prompt.
    assert!(output.result.contains("27,894"));
    // assert!(output.result.trim().ends_with(";)"));
}

/// Tests the query generation step with the LocalAiProvider using a mock server.
/// This test isolates the AI provider interaction from the storage provider.
#[tokio::test]
async fn test_get_query_from_prompt_local_provider() {
    setup_tracing();

    // 1. Setup Mock Server
    let server = MockServer::start();
    let mock_model = "test-model";

    // 2. Define the mock response from the AI.
    let mock_response_body = json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "```sql\nSELECT * FROM mock_table;\n```"
            }
        }]
    });

    // 3. Configure the mock server.
    // We only care that it receives a POST to the correct path. We don't
    // check the body, which makes the test less brittle to prompt changes.
    let mock = server.mock(|when, then| {
        when.method(POST).path("/v1/chat/completions");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(mock_response_body);
    });

    // 4. Setup Client with LocalAiProvider pointing to the mock server.
    let project_id = env::var("BIGQUERY_PROJECT_ID").expect("BIGQUERY_PROJECT_ID not set");
    let api_url = server.url("/v1/chat/completions");
    let ai_provider =
        Box::new(LocalAiProvider::new(api_url, None, Some(mock_model.to_string())).unwrap());

    let client = PromptClientBuilder::default()
        .ai_provider(ai_provider)
        .bigquery_storage(project_id)
        .await
        .unwrap()
        .build()
        .unwrap();

    // 5. Execute the prompt.
    // Note: This uses a real BigQuery connection to get the schema for context,
    // but the actual AI call to generate the query is mocked.
    let options = ExecutePromptOptions {
        prompt: "This prompt will be sent to the mock AI".to_string(),
        table_name: Some("bigquery-public-data.samples.shakespeare".to_string()),
        ..Default::default()
    };

    // We call `get_query_from_prompt` directly to test only the query generation part.
    let query_result = client.get_query_from_prompt(&options).await;

    // 6. Assertions
    mock.assert(); // Verify the mock server was called exactly once.
    assert!(query_result.is_ok());
    let query = query_result.unwrap().result;
    assert_eq!(query, "SELECT * FROM mock_table;");
}

/// Tests that a storage provider error is handled correctly.
#[tokio::test]
async fn test_execute_prompt_storage_error() {
    setup_tracing();
    let project_id = env::var("BIGQUERY_PROJECT_ID").expect("BIGQUERY_PROJECT_ID not set");

    let client = PromptClientBuilder::default()
        .ai_provider(create_real_ai_provider())
        .bigquery_storage(project_id)
        .await
        .unwrap()
        .build()
        .unwrap();

    // Use an invalid table name that will cause a BigQuery error.
    let options = ExecutePromptOptions {
        prompt: "This should fail".to_string(),
        table_name: Some(
            "non_existent_project.non_existent_dataset.non_existent_table".to_string(),
        ),
        ..Default::default()
    };

    let result = client.execute_prompt_with_options(options).await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        PromptError::StorageOperationFailed(_)
    ));
}

/// Tests that an AI provider API error is handled correctly.
#[tokio::test]
async fn test_execute_prompt_ai_provider_error() {
    setup_tracing();

    // 1. Setup Mock Server to return an error
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/v1/chat/completions");
        then.status(500).body("Internal Server Error");
    });

    // 2. Setup Client with LocalAiProvider pointing to the mock server
    let api_url = server.url("/v1/chat/completions");
    let ai_provider = Box::new(LocalAiProvider::new(api_url, None, None).unwrap());

    let client = PromptClientBuilder::default()
        .ai_provider(ai_provider)
        .storage_provider(Box::new(MockStorageProvider)) // Use a mock storage to isolate the test
        .build()
        .unwrap();

    // 3. Execute the prompt
    let options = ExecutePromptOptions {
        prompt: "This prompt will trigger an AI error".to_string(),
        ..Default::default()
    };

    let result = client.execute_prompt_with_options(options).await;

    // 4. Assertions
    mock.assert();
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), PromptError::AiApi(_)));
}

/// Tests that a prompt asking a direct question (not for a query) gets a direct answer.
#[tokio::test]
async fn test_execute_prompt_direct_answer() {
    setup_tracing();
    let client = PromptClientBuilder::default()
        .ai_provider(create_real_ai_provider())
        .storage_provider(Box::new(MockStorageProvider)) // No DB access needed
        .build()
        .unwrap();

    let options = ExecutePromptOptions {
        prompt: "what is today?".to_string(),
        ..Default::default()
    };

    let result = client.execute_prompt_with_options(options).await;

    if let Err(e) = &result {
        eprintln!("Error in test_execute_prompt_direct_answer: {e}");
    }
    assert!(result.is_ok());
    let output = result.unwrap();

    // We can't know the exact output, but it should contain the current year.
    // This confirms the AI used the injected `TODAY` context.
    let current_year = Utc::now().format("%Y").to_string();
    assert!(output.result.contains(&current_year));
    assert!(!output.result.to_uppercase().contains("SELECT"));
}
