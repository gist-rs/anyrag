//! # Common Test Utilities
//!
//! This module centralizes test harnesses and helper functions used across the
//! `anyrag-server` integration tests. It includes:
//!
//! - `TestApp`: A full application harness that spawns a real server on a random port,
//!   configured with mock external services. This is ideal for E2E tests of API endpoints.
//! - `TestSetup`: A lighter-weight setup for tests that only need to interact with
//!   a storage provider (like a temporary SQLite database) without a running server.
//! - Helper functions for creating mock data structures.

// Allow unused code because this is a test utility module, and not all
// functions might be used by every test file that includes it.
#![allow(unused)]

// By including `main.rs`, we make the binary's modules (like `state` and `router`)
// available to the test suite under the `main` namespace. This is a standard
// pattern for testing Rust binaries.
#[path = "../src/main.rs"]
pub mod main;

use anyhow::Result;
use anyrag::{
    ingest::articles::CREATE_ARTICLES_TABLE_SQL,
    providers::{
        ai::local::LocalAiProvider,
        db::{sqlite::SqliteProvider, storage::Storage},
    },
    types::{TableField, TableSchema},
    PromptClientBuilder,
};
use axum::serve;
use httpmock::MockServer;
use reqwest::Client;
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tempfile::NamedTempFile;
use tokio::{net::TcpListener, task::JoinHandle};

// --- Full Application Test Harness ---

/// A harness for end-to-end testing of the Axum server.
///
/// This struct spawns the server on a random available port, sets up a temporary
/// SQLite database, and configures the `AppState` to use a mock AI provider
/// pointed at an `httpmock::MockServer` instance.
pub struct TestApp {
    pub address: String,
    pub client: Client,
    pub mock_server: MockServer,
    pub db_path: PathBuf,
    _db_file: NamedTempFile,
    _server_handle: JoinHandle<()>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl TestApp {
    /// Spawns the application server and returns a `TestApp` instance.
    pub async fn spawn() -> Result<Self> {
        dotenvy::dotenv().ok();
        // `try_init` is used to prevent panic if the logger is already initialized.
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .compact()
            .try_init();

        let mock_server = MockServer::start();
        let db_file = NamedTempFile::new()?;
        let db_path = db_file.path().to_path_buf();

        let ai_provider = Box::new(LocalAiProvider::new(
            mock_server.url("/v1/chat/completions"),
            None,
            None,
        )?);

        let sqlite_provider = Arc::new(SqliteProvider::new(db_path.to_str().unwrap()).await?);

        // Create and populate a dummy table for testing. This is crucial for tests
        // that rely on a valid table schema being present (e.g., e2e_prompt_test).
        sqlite_provider
            .execute_query(
                "CREATE TABLE test_table (id INTEGER PRIMARY KEY, name TEXT, value REAL);",
            )
            .await
            .expect("Failed to create test table in TestApp");
        sqlite_provider
            .execute_query("INSERT INTO test_table (id, name, value) VALUES (1, 'test', 1.0)")
            .await
            .expect("Failed to insert data into test table in TestApp");

        // Also ensure the `articles` table exists, as it's needed for search endpoints.
        sqlite_provider
            .execute_query(CREATE_ARTICLES_TABLE_SQL)
            .await
            .expect("Failed to create articles table in TestApp");

        let prompt_client = Arc::new(
            PromptClientBuilder::new()
                .ai_provider(ai_provider)
                .storage_provider(Box::new(sqlite_provider.as_ref().clone()))
                .build()?,
        );

        // Build the application state, using mocks for external services.
        let app_state = main::state::AppState {
            prompt_client,
            sqlite_provider,
            embeddings_api_url: Some(mock_server.url("/v1/embeddings")),
            embeddings_model: Some("mock-embedding-model".to_string()),
            ..Default::default()
        };

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr: SocketAddr = listener.local_addr()?;
        let address = format!("http://{addr}");

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let server_handle = tokio::spawn(async move {
            let app = main::router::create_router(app_state);
            let server = serve(listener, app).with_graceful_shutdown(async {
                shutdown_rx.await.ok();
            });
            if let Err(e) = server.await {
                tracing::error!("[TestApp] Server error: {}", e);
            }
        });

        // Give the server a moment to start up.
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(Self {
            address,
            client: Client::new(),
            mock_server,
            db_path,
            _db_file: db_file,
            _server_handle: server_handle,
            shutdown_tx: Some(shutdown_tx),
        })
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            // The receiver might already be gone if the server task panicked,
            // so we ignore the result of send.
            let _ = tx.send(());
        }
    }
}

/// A default implementation for `AppState` used for convenience in tests.
///
/// The `..Default::default()` syntax in `TestApp::spawn` relies on this to fill in
/// fields like the prompt templates. The core providers (`prompt_client`, etc.) are
/// immediately overwritten with the real test instances.
impl Default for main::state::AppState {
    fn default() -> Self {
        let mock_ai =
            Box::new(LocalAiProvider::new("http://localhost".to_string(), None, None).unwrap());
        // In a test context, we often can't use async in `default`, so `block_on` is acceptable.
        let mock_storage =
            Box::new(futures::executor::block_on(SqliteProvider::new(":memory:")).unwrap());

        let prompt_client = PromptClientBuilder::new()
            .ai_provider(mock_ai)
            .storage_provider(mock_storage)
            .build()
            .unwrap();

        Self {
            prompt_client: Arc::new(prompt_client),
            sqlite_provider: Arc::new(
                futures::executor::block_on(SqliteProvider::new(":memory:")).unwrap(),
            ),
            embeddings_api_url: None,
            embeddings_model: None,
            query_system_prompt_template: None,
            query_user_prompt_template: None,
            format_system_prompt_template: None,
            format_user_prompt_template: None,
        }
    }
}

// --- Storage-Only Test Setup ---

/// A struct to hold common test setup resources for storage-related tests.
pub struct TestSetup {
    pub storage_provider: Box<dyn Storage>,
    // The temporary file for the SQLite database. It's important to keep this
    // in scope, so it's not deleted until the TestSetup is dropped.
    _db_file: NamedTempFile,
}

impl TestSetup {
    /// Creates a new test setup with a temporary SQLite provider and a test table.
    pub async fn new() -> Self {
        let db_file = NamedTempFile::new().expect("Failed to create temp file for sqlite db");
        let db_path = db_file.path().to_str().unwrap();

        let provider = SqliteProvider::new(db_path)
            .await
            .expect("Failed to create SqliteProvider");

        // Create and populate a dummy table for testing.
        provider
            .execute_query(
                "CREATE TABLE test_table (id INTEGER PRIMARY KEY, name TEXT, value REAL);",
            )
            .await
            .expect("Failed to create test table");

        provider
            .execute_query("INSERT INTO test_table (id, name, value) VALUES (1, 'test', 1.0)")
            .await
            .expect("Failed to insert data into test table");

        Self {
            storage_provider: Box::new(provider),
            _db_file: db_file,
        }
    }
}

// --- Mock Data Helpers ---

/// A helper to get a mock `TableSchema` for testing.
pub fn get_mock_schema() -> Arc<TableSchema> {
    use anyrag::types::FieldType;

    Arc::new(TableSchema {
        fields: vec![
            TableField {
                name: "id".to_string(),
                r#type: FieldType::Integer,
                description: Some("The primary key.".to_string()),
            },
            TableField {
                name: "name".to_string(),
                r#type: FieldType::String,
                description: Some("The name of the item.".to_string()),
            },
            TableField {
                name: "value".to_string(),
                r#type: FieldType::Float,
                description: Some("A floating point value.".to_string()),
            },
        ],
    })
}
