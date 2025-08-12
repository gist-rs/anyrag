//! # Content-Type Prompt Selection Tests
//!
//! This file tests that the server correctly uses specialized prompts when a
//! `content_type` is provided in the API request. It verifies that the
//! prompt selection logic flows correctly from the server to the library.

// By including the binary's main source file, we can access its public functions
// and modules for testing purposes.
#[path = "../src/main.rs"]
mod main;

use anyrag::{providers::ai::AiProvider, providers::db::sqlite::SqliteProvider, PromptError};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;

use crate::main::{router, state::AppState};

// --- Mock AI Provider for Logic Testing ---
// A mock implementation of the AiProvider trait is used to capture the prompts
// sent by the application, allowing us to assert that the correct prompt
// templates were selected based on the content type.

#[derive(Clone, Debug)]
pub struct MockAiProvider {
    pub call_history: Arc<RwLock<Vec<(String, String)>>>,
}

impl MockAiProvider {
    pub fn new(call_history: Arc<RwLock<Vec<(String, String)>>>) -> Self {
        Self { call_history }
    }
}

#[async_trait]
impl AiProvider for MockAiProvider {
    async fn generate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, PromptError> {
        self.call_history
            .write()
            .unwrap()
            .push((system_prompt.to_string(), user_prompt.to_string()));
        // The actual response doesn't matter for this test.
        Ok("Mock AI response: content type was processed.".to_string())
    }
}

/// Spawns the application with a custom AI provider to inspect prompts.
///
/// This helper function sets up a full application instance but replaces the
/// standard AI provider with a mock one. It returns the server's address.
async fn spawn_app_with_mock_ai(state: AppState) -> String {
    // Bind to a random port and start the server.
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{port}");

    tokio::spawn(async move {
        let app = router::create_router(state);
        axum::serve(listener, app).await.unwrap();
    });

    address
}

#[tokio::test]
async fn test_prompt_selects_rss_template_for_rss_content_type() {
    // --- Arrange ---
    // 1. Setup the mock AI provider and its call history tracker.
    let call_history = Arc::new(RwLock::new(Vec::new()));
    let mock_provider = MockAiProvider::new(call_history.clone());

    // 2. Build the application state with our mock provider.
    // Set dummy env vars required by the config.
    std::env::set_var("AI_API_URL", "http://mock-url.com");
    std::env::set_var("BIGQUERY_PROJECT_ID", "mock-project");
    let config = main::config::get_config().expect("Failed to read test config");
    let sqlite_provider = SqliteProvider::new(":memory:").await.unwrap();

    let prompt_client = anyrag::PromptClientBuilder::new()
        .ai_provider(Box::new(mock_provider))
        .storage_provider(Box::new(sqlite_provider))
        .build()
        .unwrap();

    let state = AppState {
        prompt_client: Arc::new(prompt_client),
        sqlite_provider: Arc::new(SqliteProvider::new(":memory:").await.unwrap()),
        embeddings_api_url: None,
        embeddings_model: None,
        query_system_prompt_template: config.query_system_prompt_template,
        query_user_prompt_template: config.query_user_prompt_template,
        format_system_prompt_template: config.format_system_prompt_template,
        format_user_prompt_template: config.format_user_prompt_template,
    };

    // 3. Spawn the app and create an HTTP client.
    let address = spawn_app_with_mock_ai(state).await;
    let client = Client::new();

    // 4. Define the payload with the RSS content type and context.
    let payload = json!({
        "prompt": "Summarize the latest articles about Rust.",
        "content_type": "rss",
        "context": "<item><title>Rust 1.78</title></item><item><title>New Axum Release</title></item>"
    });

    // 5. The expected system prompt for RSS content, as defined in `types.rs`.
    const RSS_SYSTEM_PROMPT: &str = "You are an AI assistant that specializes in analyzing and summarizing content from RSS feeds. Answer the user's question based on the provided article snippets.";

    // --- Act ---
    let response = client
        .post(format!("{address}/prompt"))
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request.");

    // --- Assert ---
    assert!(response.status().is_success());

    let history = call_history.read().unwrap();
    assert_eq!(
        history.len(),
        1,
        "Expected exactly one call to the AI provider"
    );

    let (system_prompt, user_prompt) = &history[0];

    // Assert that the correct, specialized system prompt was used.
    assert_eq!(system_prompt, RSS_SYSTEM_PROMPT);

    // Assert that the provided context and prompt were injected into the user prompt.
    assert!(user_prompt.contains("<item><title>Rust 1.78</title></item>"));
    assert!(user_prompt.contains("Summarize the latest articles about Rust."));
}
