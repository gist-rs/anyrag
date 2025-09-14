//! # Storage Management for GitHub Ingestion
//!
//! This module handles the creation and management of SQLite databases for storing
//! repository metadata and extracted code examples, as outlined in `PLAN.md`.

use super::types::{GeneratedExample, GitHubIngestError, TrackedRepository};
use crate::providers::db::sqlite::SqliteProvider;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;
use turso::params;

const META_DB_NAME: &str = "github_meta.db";

/// Manages all database interactions for the GitHub ingestion feature.
#[derive(Clone)]
pub struct StorageManager {
    /// A provider connected to the main `github_meta.db` which tracks all repositories.
    meta_db_provider: SqliteProvider,
    /// The base directory where all repository-specific databases are stored.
    db_dir: PathBuf,
}

impl StorageManager {
    /// Creates a new `StorageManager` and initializes the main metadata database.
    ///
    /// # Arguments
    /// * `base_db_dir`: The directory where all databases (meta and repo-specific) will be stored.
    pub async fn new(base_db_dir: &str) -> Result<Self, GitHubIngestError> {
        let db_dir = Path::new(base_db_dir);
        fs::create_dir_all(db_dir)?;
        let meta_db_path = db_dir.join(META_DB_NAME);

        let provider = SqliteProvider::new(meta_db_path.to_str().unwrap()).await?;
        Self::initialize_main_db(&provider).await?;

        info!(
            "StorageManager initialized. Metadata DB is at: {}",
            meta_db_path.display()
        );

        Ok(Self {
            meta_db_provider: provider,
            db_dir: db_dir.to_path_buf(),
        })
    }

    /// Adds a new repository to be tracked, creates its dedicated database and tables,
    /// and returns the tracking information.
    /// If the repository is already tracked, it returns the existing information.
    pub async fn track_repository(
        &self,
        url: &str,
    ) -> Result<TrackedRepository, GitHubIngestError> {
        let repo_name = Self::url_to_repo_name(url);
        let db_filename = format!("{repo_name}.db");
        let db_path = self.db_dir.join(&db_filename);
        let db_path_str = db_path.to_str().unwrap().to_string();

        let conn = self.meta_db_provider.db.connect()?;

        // Check if the repository is already tracked.
        let mut rows = conn
            .query(
                "SELECT repo_name, url, db_path FROM repositories WHERE url = ?",
                params![url],
            )
            .await?;

        if let Some(row) = rows.next().await? {
            info!(
                "Repository '{}' is already tracked. Returning existing info.",
                url
            );
            return Ok(TrackedRepository {
                repo_name: row.get(0)?,
                url: row.get(1)?,
                db_path: row.get(2)?,
            });
        }

        info!(
            "First time tracking repository '{}'. Initializing new database.",
            url
        );
        // Initialize the dedicated database for the new repository.
        let repo_provider = SqliteProvider::new(&db_path_str).await?;
        Self::initialize_repo_db(&repo_provider).await?;

        // Add the new repository to the metadata database.
        conn.execute(
            "INSERT INTO repositories (repo_name, url, db_path) VALUES (?, ?, ?)",
            params![repo_name.clone(), url, db_path_str.clone()],
        )
        .await?;

        Ok(TrackedRepository {
            repo_name,
            url: url.to_string(),
            db_path: db_path_str,
        })
    }

    /// Stores a batch of extracted examples using a "delete then insert" strategy for idempotency.
    pub async fn store_examples(
        &self,
        repo: &TrackedRepository,
        examples: Vec<GeneratedExample>,
    ) -> Result<usize, GitHubIngestError> {
        if examples.is_empty() {
            return Ok(0);
        }

        let version = &examples[0].version;
        info!(
            "Storing {} examples for repo '{}', version '{}'",
            examples.len(),
            repo.repo_name,
            version
        );

        let provider = SqliteProvider::new(&repo.db_path).await?;
        let conn = provider.db.connect()?;
        conn.execute("BEGIN TRANSACTION", ()).await?;

        // 1. Delete all existing examples for this specific version.
        info!(
            "Deleting existing examples for version '{}' before insertion.",
            version
        );
        conn.execute(
            "DELETE FROM generated_examples WHERE version = ?",
            params![version.clone()],
        )
        .await?;

        // 2. Insert all the new examples.
        let mut stmt = conn.prepare(
            "INSERT INTO generated_examples (example_handle, content, source_file, source_type, version)
             VALUES (?, ?, ?, ?, ?)"
        ).await?;

        for example in &examples {
            stmt.execute(params![
                example.example_handle.clone(),
                example.content.clone(),
                example.source_file.clone(),
                example.source_type.to_string(),
                example.version.clone()
            ])
            .await?;
        }

        conn.execute("COMMIT", ()).await?;
        info!(
            "Successfully stored {} examples for version '{}'.",
            examples.len(),
            version
        );
        Ok(examples.len())
    }

    /// Generates and stores embeddings for examples that don't have them yet.
    pub async fn embed_and_store_examples(
        &self,
        repo: &TrackedRepository,
        api_url: &str,
        model_name: &str,
    ) -> Result<usize, GitHubIngestError> {
        info!(
            "Starting embedding process for repo '{}' with model '{}'",
            repo.repo_name, model_name
        );

        let provider = SqliteProvider::new(&repo.db_path).await?;
        let conn = provider.db.connect()?;

        // Select examples that are not yet in the embeddings table.
        // We need the ID for the foreign key and the content for embedding.
        let mut stmt = conn
            .prepare(
                "SELECT ge.id, ge.content FROM generated_examples ge
             LEFT JOIN example_embeddings ee ON ge.id = ee.example_id
             WHERE ee.id IS NULL",
            )
            .await?;
        let mut rows = stmt.query(()).await?;

        let mut embed_count = 0;
        while let Some(row) = rows.next().await? {
            let example_id: i64 = row.get(0)?;
            let content: String = row.get(1)?;

            let vector = crate::providers::ai::generate_embedding(api_url, model_name, &content)
                .await
                .map_err(|e| GitHubIngestError::Internal(e.into()))?;

            let vector_bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(vector.as_ptr() as *const u8, vector.len() * 4)
            };

            conn.execute(
                "INSERT INTO example_embeddings (example_id, model_name, embedding) VALUES (?, ?, ?)",
                params![example_id, model_name.to_string(), vector_bytes],
            )
            .await?;
            embed_count += 1;
        }

        info!(
            "Embedding complete. Processed {} new examples.",
            embed_count
        );
        Ok(embed_count)
    }

    /// Retrieves all examples for a specific repository and version.
    pub async fn get_examples(
        &self,
        repo_name: &str,
        version: &str,
    ) -> Result<Vec<GeneratedExample>, GitHubIngestError> {
        // 1. Find the repo's db_path from the meta db.
        let conn = self.meta_db_provider.db.connect()?;
        let mut rows = conn
            .query(
                "SELECT db_path FROM repositories WHERE repo_name = ?",
                params![repo_name],
            )
            .await?;

        let db_path: String = if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            return Err(GitHubIngestError::Config(format!(
                "Repository '{repo_name}' not found in metadata."
            )));
        };

        // 2. Connect to the repo-specific DB.
        let provider = SqliteProvider::new(&db_path).await?;
        let repo_conn = provider.db.connect()?;

        // 3. Query for examples.
        let mut stmt = repo_conn
            .prepare(
                "SELECT example_handle, content, source_file, source_type, version FROM generated_examples WHERE version = ?",
            )
            .await?;
        let mut example_rows = stmt.query(params![version]).await?;

        let mut examples = Vec::new();
        while let Some(row) = example_rows.next().await? {
            let source_type_str: String = row.get(3)?;
            let source_type = match source_type_str.as_str() {
                "readme" => super::types::ExampleSourceType::Readme,
                "example_file" => super::types::ExampleSourceType::ExampleFile,
                "doc_comment" => super::types::ExampleSourceType::DocComment,
                "test" => super::types::ExampleSourceType::Test,
                _ => {
                    info!(
                        "Skipping example with unknown source type: {}",
                        source_type_str
                    );
                    continue;
                }
            };

            examples.push(GeneratedExample {
                example_handle: row.get(0)?,
                content: row.get(1)?,
                source_file: row.get(2)?,
                source_type,
                version: row.get(4)?,
            });
        }

        Ok(examples)
    }

    /// Retrieves a `SqliteProvider` for a specific repository.
    pub async fn get_provider_for_repo(
        &self,
        repo_name: &str,
    ) -> Result<SqliteProvider, GitHubIngestError> {
        let conn = self.meta_db_provider.db.connect()?;
        let mut rows = conn
            .query(
                "SELECT db_path FROM repositories WHERE repo_name = ?",
                params![repo_name],
            )
            .await?;

        let db_path: String = if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            return Err(GitHubIngestError::Config(format!(
                "Repository '{repo_name}' not found in metadata."
            )));
        };

        let provider = SqliteProvider::new(&db_path).await?;
        Ok(provider)
    }

    /// Retrieves the latest version string for a given repository.
    pub async fn get_latest_version(
        &self,
        repo_name: &str,
    ) -> Result<Option<String>, GitHubIngestError> {
        let repo_provider = self.get_provider_for_repo(repo_name).await?;
        let conn = repo_provider.db.connect()?;

        let mut rows = conn
            .query(
                "SELECT version FROM generated_examples ORDER BY created_at DESC LIMIT 1",
                (),
            )
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    // --- Private Helper Functions ---

    /// Creates the `repositories` table in the main metadata database if it doesn't exist.
    async fn initialize_main_db(provider: &SqliteProvider) -> Result<(), GitHubIngestError> {
        let conn = provider.db.connect()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS repositories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_name TEXT NOT NULL UNIQUE,
                url TEXT NOT NULL UNIQUE,
                db_path TEXT NOT NULL UNIQUE,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            (),
        )
        .await?;
        Ok(())
    }

    /// Creates the necessary tables (`generated_examples`, `example_embeddings`)
    /// in a repository-specific database.
    async fn initialize_repo_db(provider: &SqliteProvider) -> Result<(), GitHubIngestError> {
        let conn = provider.db.connect()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS generated_examples (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                example_handle TEXT NOT NULL UNIQUE,
                content TEXT NOT NULL,
                source_file TEXT NOT NULL,
                source_type TEXT NOT NULL,
                version TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            (),
        )
        .await?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS example_embeddings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                example_id INTEGER NOT NULL,
                model_name TEXT NOT NULL,
                embedding BLOB NOT NULL,
                FOREIGN KEY (example_id) REFERENCES generated_examples(id) ON DELETE CASCADE
            )",
            (),
        )
        .await?;
        Ok(())
    }

    /// Sanitizes a GitHub URL to create a filesystem-friendly repository name.
    /// e.g., "https://github.com/tursodatabase/turso" -> "tursodatabase-turso"
    pub fn url_to_repo_name(url: &str) -> String {
        let name = url
            .trim_end_matches('/')
            .split('/')
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .take(2)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("-");

        // Final sanitization for any other invalid characters.
        name.chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect()
    }
}
