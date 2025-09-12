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

pub mod pdf_helper;

use anyhow::Result;
use anyrag::{
    graph::types::MemoryKnowledgeGraph,
    providers::db::{sqlite::SqliteProvider, storage::Storage},
    types::{TableField, TableSchema},
};
use anyrag_server::{
    auth::middleware::Claims,
    config, router,
    state::{build_app_state, AppState},
};
use axum::serve;
use httpmock::MockServer;
use jsonwebtoken::{encode, EncodingKey, Header};
use reqwest::Client;
use std::{
    fs::File,
    io::Write,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};
use tempfile::{tempdir, NamedTempFile, TempDir};
use tokio::{net::TcpListener, task::JoinHandle};
use uuid::Uuid;

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
    pub app_state: AppState,
    pub knowledge_graph: Arc<RwLock<MemoryKnowledgeGraph>>,
    _db_file: Option<NamedTempFile>,
    _config_dir: Option<TempDir>,
    _server_handle: JoinHandle<()>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl TestApp {
    /// Spawns the application server and returns a `TestApp` instance.
    pub async fn spawn() -> Result<Self> {
        let mock_server = MockServer::start();
        let db_file = NamedTempFile::new()?;
        let db_path = db_file.path().to_path_buf();

        let config_dir = tempdir()?;
        let config_path = config_dir.path().join("config.yml");
        println!("[TestApp::spawn] CONFIGURING with DB path: {db_path:?}");
        let config_content = format!(
            r#"
port: 0
db_url: "{}"
embedding:
  api_url: "{}"
  model_name: "mock-embedding-model"
temporal_reasoning:
  keywords: ["newest", "latest", "most recent"]
  property_name: "release_date"
providers:
  gemini_default:
    provider: "local"
    api_url: "{}"
    api_key: null
    model_name: "mock-chat-model"
"#,
            db_path.to_str().unwrap(),
            mock_server.url("/v1/embeddings"),
            mock_server.url("/v1/chat/completions")
        );
        let mut file = File::create(&config_path)?;
        file.write_all(config_content.as_bytes())?;

        let config = config::get_config(Some(config_path.to_str().unwrap()))?;
        let app_state = build_app_state(config).await?;
        app_state.sqlite_provider.initialize_schema().await?;

        let mut app = TestApp::spawn_with_state(app_state, mock_server).await?;
        app._db_file = Some(db_file);
        app._config_dir = Some(config_dir);
        Ok(app)
    }

    pub async fn spawn_with_state(app_state: AppState, mock_server: MockServer) -> Result<Self> {
        dotenvy::dotenv().ok();
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .compact()
            .try_init();

        let db_path = PathBuf::from(&app_state.config.db_url);
        let knowledge_graph_clone = app_state.knowledge_graph.clone();
        let app_state_for_harness = app_state.clone();

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

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(Self {
            address,
            client: Client::new(),
            mock_server,
            db_path,
            app_state: app_state_for_harness,
            knowledge_graph: knowledge_graph_clone,
            _db_file: None,
            _config_dir: None,
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

// --- Test Data Builder ---

/// A fluent builder for creating test data in the database.
pub struct TestDataBuilder<'a> {
    // We hold a reference to the TestApp to ensure the database file outlives the builder.
    _app: &'a TestApp,
    conn: turso::Connection,
}

impl<'a> TestDataBuilder<'a> {
    /// Creates a new TestDataBuilder.
    pub async fn new(app: &'a TestApp) -> Result<Self> {
        let db = turso::Builder::new_local(app.db_path.to_str().unwrap())
            .build()
            .await?;
        let conn = db.connect()?;
        Ok(Self { _app: app, conn })
    }

    /// Adds a document to the database.
    pub async fn add_document(
        &self,
        doc_id: &str,
        owner_id: &str,
        title: &str,
        content: &str,
        source_url: Option<&str>,
    ) -> Result<&Self> {
        let default_url = format!("http://test.com/{doc_id}");
        let final_source_url = source_url.unwrap_or(&default_url);
        self.conn.execute(
            "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?, ?, ?, ?, ?)",
            turso::params![doc_id, owner_id, final_source_url, title, content],
        )
        .await?;
        Ok(self)
    }

    /// Adds an FAQ item to the database.
    pub async fn add_faq(
        &self,
        doc_id: &str,
        owner_id: &str,
        question: &str,
        answer: &str,
    ) -> Result<&Self> {
        self.conn.execute(
            "INSERT INTO faq_items (document_id, owner_id, question, answer) VALUES (?, ?, ?, ?)",
            turso::params![doc_id, owner_id, question, answer],
        )
        .await?;
        Ok(self)
    }

    /// Adds metadata to a document.
    pub async fn add_metadata(
        &self,
        doc_id: &str,
        owner_id: &str,
        meta_type: &str, // e.g., ENTITY, KEYPHRASE
        subtype: &str,   // e.g., PRODUCT, CONCEPT
        value: &str,
    ) -> Result<&Self> {
        self.conn.execute(
            "INSERT INTO content_metadata (document_id, owner_id, metadata_type, metadata_subtype, metadata_value) VALUES (?, ?, ?, ?, ?)",
            turso::params![doc_id, owner_id, meta_type, subtype, value],
        )
        .await?;
        Ok(self)
    }

    /// Adds an embedding vector to a document.
    pub async fn add_embedding(&self, doc_id: &str, vector: Vec<f32>) -> Result<&Self> {
        let vector_bytes: &[u8] =
            unsafe { std::slice::from_raw_parts(vector.as_ptr() as *const u8, vector.len() * 4) };
        self.conn
            .execute(
                "INSERT INTO document_embeddings (document_id, model_name, embedding) VALUES (?, ?, ?)",
                turso::params![doc_id, "mock-embedding-model", vector_bytes],
            )
            .await?;
        Ok(self)
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
/// Generates a valid JWT for a given user identifier (subject).
pub fn generate_jwt(sub: &str) -> Result<String> {
    generate_jwt_with_expiry(sub, 3600)
}

/// Generates a valid JWT for a given user identifier (subject) with a custom expiration.
pub fn generate_jwt_with_expiry(sub: &str, expires_in_secs: u64) -> Result<String> {
    let expiration = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + expires_in_secs;
    let user_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, sub.as_bytes()).to_string();
    let claims = Claims {
        sub: sub.to_string(),
        exp: expiration as usize,
        user_id,
    };
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "a-secure-secret-key".to_string());
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )?;
    Ok(token)
}
