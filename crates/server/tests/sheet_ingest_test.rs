//! # Google Sheet Ingestion and Prompting E2E Test
//!
//! This file contains a full end-to-end integration test for the feature
//! that allows users to prompt the server with a Google Sheet URL. It verifies
//! the entire workflow from URL detection to final, formatted output.

use anyhow::Result;
use anyrag::{
    providers::{
        ai::local::LocalAiProvider,
        db::{sqlite::SqliteProvider, storage::Storage},
    },
    PromptClient, PromptClientBuilder, PromptError,
};
use async_trait::async_trait;
use gcp_bigquery_client::model::table_schema::TableSchema;
use httpmock::prelude::*;
use reqwest::Client;
use serde_json::json;
use std::fmt::Debug;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};
use tracing::info;
use turso::Value as TursoValue;

// Include the binary's main source file to access its components.
#[path = "../src/main.rs"]
mod main;

// --- Mock Storage Provider for Test Isolation ---
// This mock is used in the prompt client for the final formatting step.
// It simulates the database result after the query has been generated and executed.
#[derive(Clone, Debug)]
pub struct MockStorageProvider;

#[async_trait]
impl Storage for MockStorageProvider {
    fn name(&self) -> &str {
        "MockDB"
    }
    fn language(&self) -> &str {
        "SQL"
    }
    async fn execute_query(&self, _query: &str) -> Result<String, PromptError> {
        // This simulates the result of the `SELECT COUNT(*)` query.
        Ok("[{\"count\":3}]".to_string())
    }
    async fn get_table_schema(&self, _table_name: &str) -> Result<Arc<TableSchema>, PromptError> {
        // The schema isn't needed for the formatting step, so an empty one is fine.
        Ok(Arc::new(TableSchema::new(vec![])))
    }
}

/// Spawns the application in the background for testing, using a shared provider.
async fn spawn_app_for_sheet_test(
    prompt_client: Arc<PromptClient>,
    sqlite_provider: Arc<SqliteProvider>,
) -> Result<(
    String,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
)> {
    info!("[spawn_app] Creating AppState.");
    let app_state = main::AppState {
        prompt_client,
        sqlite_provider,
        embeddings_api_url: None,
        embeddings_model: None,
        query_system_prompt_template: None,
        query_user_prompt_template: None,
        format_system_prompt_template: None,
        format_user_prompt_template: None,
    };
    let app = main::create_router(app_state);

    info!("[spawn_app] Binding TCP listener.");
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{port}");
    info!("[spawn_app] Listener bound to: {}", address);

    let (tx, rx) = tokio::sync::oneshot::channel();
    let server_handle = tokio::spawn(async move {
        let server = axum::serve(listener, app).with_graceful_shutdown(async {
            rx.await.ok();
            info!("[spawn_app] Graceful shutdown signal received.");
        });
        if let Err(e) = server.await {
            eprintln!("[spawn_app] Server error: {e}");
        }
        info!("[spawn_app] Server task finished.");
    });

    sleep(Duration::from_millis(100)).await;
    info!("[spawn_app] Spawn complete.");
    Ok((address, tx, server_handle))
}

#[tokio::test]
async fn test_sheet_ingestion_and_prompting_workflow() -> Result<()> {
    // --- 1. Arrange ---
    info!("[test] Starting test_sheet_ingestion_and_prompting_workflow");
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();
    let mock_server = MockServer::start();
    let http_client = Client::new();

    let temp_db_file = NamedTempFile::new()?;
    let db_path = temp_db_file.path();
    let sqlite_provider =
        Arc::new(SqliteProvider::new(db_path.to_str().expect("Path is not valid UTF-8")).await?);
    info!(
        "[test] File-based SQLite provider created at: {:?}",
        db_path
    );

    let sheet_path = "/spreadsheets/d/1Upsr6r6ufkYougDFVBQOQNgNf9Syrwv2CTNhFbVNu2w/edit";
    let expected_table_name = "spreadsheets_1Upsr6r6ufkYougDFVBQOQNgNf9Syrwv2CTNhFbVNu2w";

    // --- 2. Mock Services ---
    info!("[test] Setting up mocks for external services.");
    let mock_csv_content =
        "Name,Role,Team\nAlice,Engineer,Alpha\nBob,Designer,Bravo\nCharlie,PM,Alpha";
    let download_mock = mock_server.mock(|when, then| {
        when.method(GET)
            .path("/spreadsheets/d/1Upsr6r6ufkYougDFVBQOQNgNf9Syrwv2CTNhFbVNu2w/export")
            .query_param("format", "csv");
        then.status(200).body(mock_csv_content);
    });

    // Mock for the FIRST AI call (Query Generation).
    // We make it specific by checking for content unique to the query generation prompt.
    let query_gen_mock = mock_server.mock(|when, then| {
        when.method(POST)
            .path("/v1/chat/completions")
            .body_contains("expert for"); // Unique to DEFAULT_QUERY_SYSTEM_PROMPT
        then.status(200).json_body(json!({
            "choices": [{
                "message": { "role": "assistant", "content": format!("```sql\nSELECT COUNT(*) as count FROM {};\n```", expected_table_name) }
            }]
        }));
    });

    // Mock for the SECOND AI call (Response Formatting).
    // We make it specific by checking for content unique to the formatting prompt.
    let format_mock = mock_server.mock(|when, then| {
        when.method(POST)
            .path("/v1/chat/completions")
            .body_contains("helpful AI assistant"); // Unique to DEFAULT_FORMAT_SYSTEM_PROMPT
        then.status(200).json_body(json!({
            "choices": [{
                "message": { "role": "assistant", "content": "The sheet has 3 records." }
            }]
        }));
    });
    info!("[test] Mocks configured.");

    // --- 3. Spawn App and Send Request ---
    // This prompt client is used inside the app state. It uses the real SQLite provider
    // for the ingestion part, but its AI provider points to our mock server.
    // The storage provider is also mocked for the final step to avoid needing BigQuery.
    let ai_provider = LocalAiProvider::new(
        mock_server.url("/v1/chat/completions"),
        None,
        Some("mock-model".to_string()),
    )?;
    let prompt_client = Arc::new(
        PromptClientBuilder::new()
            .ai_provider(Box::new(ai_provider))
            .storage_provider(Box::new(MockStorageProvider)) // Mock storage for the formatting step.
            .build()?,
    );

    let (app_address, shutdown_tx, server_handle) =
        spawn_app_for_sheet_test(prompt_client.clone(), sqlite_provider.clone()).await?;
    info!("[test] App spawned at {}.", app_address);

    let payload = json!({
        "prompt": format!("Count the records in this sheet: {}", mock_server.url(sheet_path)),
        "instruction": "Provide a natural language summary."
    });

    info!("[test] Sending POST request to /prompt.");
    let response = http_client
        .post(format!("{}/prompt", app_address))
        .json(&payload)
        .send()
        .await?
        .error_for_status()?;
    info!("[test] Received response from /prompt.");

    // --- 4. Assert Server Response ---
    info!("[test] Asserting server response.");
    let result_body: serde_json::Value = response.json().await?;
    let result_str = result_body["result"].as_str().unwrap();
    assert!(
        result_str.contains("3 records"),
        "The final response did not contain the formatted text '3 records'. Got: {}",
        result_str
    );
    info!("[test] Server response is correct: '{}'", result_str);

    // --- 5. Assert Database State ---
    info!("[test] Asserting database state.");
    let conn = sqlite_provider.db.connect()?;
    let mut stmt = conn
        .prepare(&format!("SELECT COUNT(*) FROM {}", expected_table_name))
        .await?;
    let mut rows = stmt.query(()).await?;
    let row = rows.next().await?.expect("COUNT(*) returned no rows");
    let count: i64 = match row.get_value(0)? {
        TursoValue::Integer(i) => i,
        _ => panic!("Expected integer for count"),
    };
    assert_eq!(count, 3, "Database count verification failed.");
    info!("[test] Database state is correct. Found {} records.", count);

    // --- 6. Assert Mocks and Shutdown ---
    info!("[test] Asserting mock calls.");
    download_mock.assert();
    query_gen_mock.assert();
    format_mock.assert();
    info!("[test] Mock calls verified.");

    info!("[test] Sending shutdown signal.");
    let _ = shutdown_tx.send(());
    server_handle.await?;
    info!("[test] Test assertions passed and server shut down gracefully. Test finished.");

    Ok(())
}
