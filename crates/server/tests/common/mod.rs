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

use anyhow::Result;
use anyrag::{
    graph::types::MemoryKnowledgeGraph,
    providers::db::{sqlite::SqliteProvider, storage::Storage},
    types::{TableField, TableSchema},
};
use anyrag_server::{
    auth::middleware::Claims,
    config::{AppConfig, EmbeddingConfig, ProviderConfig, TaskConfig},
    router,
    state::{build_app_state, AppState},
};
use axum::serve;
use httpmock::MockServer;
use jsonwebtoken::{encode, EncodingKey, Header};
use reqwest::Client;
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};
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
    pub knowledge_graph: Arc<RwLock<MemoryKnowledgeGraph>>,
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

        let sqlite_provider = SqliteProvider::new(db_path.to_str().unwrap()).await?;
        sqlite_provider.initialize_schema().await?;

        // Create a mock AppConfig that points to our mock server
        let mock_config = AppConfig {
            port: 0,
            db_url: db_path.to_str().unwrap().to_string(),
            embedding: EmbeddingConfig {
                api_url: mock_server.url("/v1/embeddings"),
                model_name: "mock-embedding-model".to_string(),
            },
            providers: HashMap::from([(
                "mock_provider".to_string(),
                ProviderConfig {
                    provider: "local".to_string(),
                    api_url: mock_server.url("/v1/chat/completions"),
                    api_key: None,
                    model_name: "mock-chat-model".to_string(),
                },
            )]),
            tasks: HashMap::from([
                (
                    "query_generation".to_string(),
                    TaskConfig {
                        provider: "mock_provider".to_string(),
                        system_prompt: "You are an SQL expert for MockDB.".to_string(),
                        user_prompt: "Context: {context}\nQuestion: {prompt}".to_string(),
                    },
                ),
                (
                    "rag_synthesis".to_string(),
                    TaskConfig {
                        provider: "mock_provider".to_string(),
                        system_prompt: "You are a strict, factual AI.".to_string(),
                        user_prompt: "User Question: {prompt}\nContext: {context}".to_string(),
                    },
                ),
                (
                    "knowledge_distillation".to_string(),
                    TaskConfig {
                        provider: "mock_provider".to_string(),
                        system_prompt: "You are an expert data extraction agent.".to_string(),
                        user_prompt: "Content: {markdown_content}".to_string(),
                    },
                ),
                (
                    "query_analysis".to_string(),
                    TaskConfig {
                        provider: "mock_provider".to_string(),
                        system_prompt: "You are an expert query analyst.".to_string(),
                        user_prompt: "Query: {prompt}".to_string(),
                    },
                ),
                (
                    "llm_rerank".to_string(),
                    TaskConfig {
                        provider: "mock_provider".to_string(),
                        system_prompt: "You are an expert search result re-ranker.".to_string(),
                        user_prompt: "Query: {query_text}\nArticles: {articles_context}"
                            .to_string(),
                    },
                ),
                (
                    "rss_summarization".to_string(),
                    TaskConfig {
                        provider: "mock_provider".to_string(),
                        system_prompt: "You are an AI assistant that specializes in analyzing and summarizing content from RSS feeds. Answer the user's question based on the provided article snippets.".to_string(),
                        user_prompt: "# User Question\n{prompt}\n\n# Article Content\n{context}".to_string(),
                    },
                ),
                (
                    "knowledge_augmentation".to_string(),
                    TaskConfig {
                        provider: "mock_provider".to_string(),
                        system_prompt: "You are an expert content analyst.".to_string(),
                        user_prompt: "Content Chunks to Analyze: {batched_content}".to_string(),
                    },
                ),
            ])
        };

        // Build the application state from the mock config.
        let app_state = build_app_state(mock_config).await?;
        let knowledge_graph_clone = app_state.knowledge_graph.clone();

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr: SocketAddr = listener.local_addr()?;
        let address = format!("http://{addr}");

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let server_handle = tokio::spawn(async move {
            let app = router::create_router(app_state);
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
            knowledge_graph: knowledge_graph_clone,
            _db_file: db_file,
            _server_handle: server_handle,
            shutdown_tx: Some(shutdown_tx),
        })
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// A struct to hold common test setup resources for storage-related tests.
pub struct TestSetup {
    pub storage_provider: Box<dyn Storage>,
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

/// Generates a valid JWT for a given user identifier (subject).
pub fn generate_jwt(sub: &str) -> Result<String> {
    let expiration = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 3600;
    let claims = Claims {
        sub: sub.to_string(),
        exp: expiration as usize,
    };
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "a-secure-secret-key".to_string());
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )?;
    Ok(token)
}
