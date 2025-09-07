use crate::types::{FieldType, TableField, TableSchema};
use crate::{
    errors::PromptError,
    providers::db::storage::{KeywordSearch, MetadataSearch, Storage, VectorSearch},
    search::SearchError,
    types::SearchResult,
};
use async_trait::async_trait;
#[cfg(feature = "core-access")]
use core_access::GUEST_USER_IDENTIFIER;
use serde_json::Value;
use std::{
    collections::HashMap,
    fmt::{self, Debug},
    sync::Arc,
};
use tokio::sync::RwLock;
use tracing::{debug, info};
use turso::{Database, Value as TursoValue};

#[cfg(feature = "core-access")]
use uuid::Uuid;

mod sql;

/// Represents a search result from the `faq_kb` table, used for RAG context.
#[derive(Debug)]
pub struct FaqSearchResult {
    pub answer: String,
    pub score: f64,
}

/// A provider for interacting with a local SQLite database using Turso.
///
/// This provider holds a `Database` instance, which manages a connection pool.
/// When cloned, it shares the same underlying database, allowing for concurrent and
/// shared access to the same database file or in-memory instance.
#[derive(Clone)]
pub struct SqliteProvider {
    /// The Turso database instance. It's cloneable and thread-safe.
    pub db: Database,
    schema_cache: Arc<RwLock<HashMap<String, Arc<TableSchema>>>>,
}

impl SqliteProvider {
    /// Creates a new `SqliteProvider` from a file path or in-memory.
    ///
    /// # Arguments
    ///
    /// * `db_path`: The path to the SQLite database file. Use ":memory:" for a unique,
    ///   isolated in-memory database. To share an in-memory database across multiple
    ///   `SqliteProvider` instances (e.g., in tests), create one provider and
    ///   then `.clone()` it.
    pub async fn new(db_path: &str) -> Result<Self, PromptError> {
        // The turso builder creates a new database instance.
        // If `db_path` is ":memory:", it's a new, isolated in-memory DB.
        // If it's a file path, it points to that file.
        let db = turso::Builder::new_local(db_path)
            .build()
            .await
            .map_err(|e| PromptError::StorageConnection(e.to_string()))?;

        // Enable WAL mode for better concurrency. This is beneficial for file-based databases.
        // It has no effect on in-memory databases but is safe to run.
        let conn = db
            .connect()
            .map_err(|e| PromptError::StorageConnection(e.to_string()))?;
        // Use `query` for PRAGMA statements that return a value to avoid "unexpected row" errors.
        conn.query("PRAGMA journal_mode=WAL;", ())
            .await
            .map_err(|e| PromptError::StorageConnection(e.to_string()))?;

        Ok(Self {
            db,
            schema_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// A helper for tests to pre-populate data by executing multiple SQL statements.
    pub async fn initialize_with_data(&self, init_sql: &str) -> Result<(), PromptError> {
        // Get a new connection for this operation.
        let conn = self
            .db
            .connect()
            .map_err(|e| PromptError::StorageConnection(e.to_string()))?;

        for statement in init_sql.split(';').filter(|s| !s.trim().is_empty()) {
            conn.execute(statement, ())
                .await
                .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?;
        }
        Ok(())
    }

    /// Ensures that all required application tables and indexes exist.
    /// This function is idempotent and safe to call on every application startup.
    pub async fn initialize_schema(&self) -> Result<(), PromptError> {
        let conn = self
            .db
            .connect()
            .map_err(|e| PromptError::StorageConnection(e.to_string()))?;

        for statement in sql::ALL_TABLE_CREATION_SQL {
            conn.execute(statement, ())
                .await
                .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?;
        }
        Ok(())
    }
}

impl Debug for SqliteProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SqliteProvider").finish_non_exhaustive()
    }
}

impl AsRef<Database> for SqliteProvider {
    fn as_ref(&self) -> &Database {
        &self.db
    }
}

/// Converts a Turso value to a serde_json::Value.
fn turso_value_to_json(v: TursoValue) -> Value {
    match v {
        TursoValue::Null => Value::Null,
        TursoValue::Integer(i) => Value::Number(i.into()),
        TursoValue::Real(f) => serde_json::Number::from_f64(f)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        TursoValue::Text(s) => Value::String(s),
        TursoValue::Blob(_) => Value::String("<blob>".to_string()),
    }
}

#[async_trait]
impl Storage for SqliteProvider {
    fn name(&self) -> &str {
        "SQLite"
    }

    fn language(&self) -> &str {
        "SQL"
    }

    /// Executes a query on SQLite and returns the result as a JSON string.
    async fn execute_query(&self, query: &str) -> Result<String, PromptError> {
        debug!(query = %query, "--> Executing SQLite query");

        // Get a new connection for this query.
        let conn = self
            .db
            .connect()
            .map_err(|e| PromptError::StorageConnection(e.to_string()))?;

        let mut stmt = conn
            .prepare(query)
            .await
            .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?;

        let column_names: Vec<String> = stmt
            .columns()
            .iter()
            .map(|c| c.name().to_string())
            .collect();

        let mut rows = stmt
            .query(())
            .await
            .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?;

        let mut json_results: Vec<Value> = Vec::new();

        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?
        {
            let mut row_map = serde_json::Map::new();
            for (i, name) in column_names.iter().enumerate() {
                let value = row
                    .get_value(i)
                    .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?;
                row_map.insert(name.clone(), turso_value_to_json(value));
            }
            json_results.push(Value::Object(row_map));
        }

        Ok(serde_json::to_string(&json_results)?)
    }

    /// Retrieves the schema for a given SQLite table.
    async fn get_table_schema(&self, table_name: &str) -> Result<Arc<TableSchema>, PromptError> {
        if let Some(schema) = self.schema_cache.read().await.get(table_name) {
            debug!(table_name = %table_name, "Returning cached schema.");
            return Ok(schema.clone());
        }
        debug!(table_name = %table_name, "Schema not in cache. Fetching from DB.");

        let conn = self
            .db
            .connect()
            .map_err(|e| PromptError::StorageConnection(e.to_string()))?;

        let query = format!("PRAGMA table_info({table_name});");
        let mut rows = conn
            .query(&query, ())
            .await
            .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?;

        let mut fields = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?
        {
            // PRAGMA table_info columns: cid, name, type, notnull, dflt_value, pk
            if let (Ok(TursoValue::Text(name)), Ok(TursoValue::Text(type_str))) =
                (row.get_value(1), row.get_value(2))
            {
                let field_type = match type_str.to_uppercase().as_str() {
                    "INTEGER" => FieldType::Integer,
                    "TEXT" => FieldType::String,
                    "REAL" => FieldType::Float,
                    "BLOB" => FieldType::Bytes,
                    "DATETIME" | "TIMESTAMP" => FieldType::Timestamp,
                    "DATE" => FieldType::Date,
                    // NUMERIC and other types default to String for simplicity
                    _ => FieldType::String,
                };

                fields.push(TableField {
                    name,
                    r#type: field_type,
                    description: None, // SQLite PRAGMA doesn't provide column comments.
                });
            }
        }

        if fields.is_empty() {
            return Err(PromptError::StorageOperationFailed(format!(
                "Table '{table_name}' not found or has no columns."
            )));
        }

        info!(table_name = %table_name, "Successfully fetched schema with {} columns.", fields.len());

        let schema = Arc::new(TableSchema { fields });

        self.schema_cache
            .write()
            .await
            .insert(table_name.to_string(), schema.clone());

        Ok(schema)
    }

    async fn list_tables(&self) -> Result<Vec<String>, PromptError> {
        info!("Listing all tables in SQLite database.");
        let conn = self
            .db
            .connect()
            .map_err(|e| PromptError::StorageConnection(e.to_string()))?;

        let mut rows = conn
            .query(
                "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%';",
                (),
            )
            .await
            .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?;

        let mut tables = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?
        {
            if let Ok(TursoValue::Text(name)) = row.get_value(0) {
                tables.push(name);
            }
        }
        Ok(tables)
    }
}

#[async_trait]
impl VectorSearch for SqliteProvider {
    /// Performs a vector similarity search using SQLite with the vss-lite extension.
    async fn vector_search(
        &self,
        query_vector: Vec<f32>,
        limit: u32,
        owner_id: Option<&str>,
        document_ids: Option<&[String]>,
    ) -> Result<Vec<SearchResult>, SearchError> {
        info!("Executing SQLite vector search on documents.");
        let conn = self.db.connect()?;

        let vector_str = format!(
            "vector('[{}]')",
            query_vector
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );

        let distance_calculation =
            format!("(1.0 - (vector_distance_cos(de.embedding, {vector_str}) / 2.0))");

        let mut sql = format!(
            "SELECT d.title, d.source_url, d.content, {distance_calculation} AS similarity
             FROM document_embeddings de
             JOIN documents d ON d.id = de.document_id"
        );

        let mut conditions: Vec<String> = vec!["de.embedding IS NOT NULL".to_string()];
        let mut query_params: Vec<TursoValue> = Vec::new();

        // This block needs the GUEST_USER_IDENTIFIER, so it's conditionally compiled.
        #[cfg(feature = "core-access")]
        {
            let guest_user_id =
                Uuid::new_v5(&Uuid::NAMESPACE_URL, GUEST_USER_IDENTIFIER.as_bytes()).to_string();

            if let Some(owner) = owner_id {
                if owner == guest_user_id {
                    // It's the guest user, they only see guest content.
                    conditions.push("d.owner_id = ?".to_string());
                    query_params.push(guest_user_id.into());
                } else {
                    // It's an authenticated user, they see their own content and guest content.
                    conditions.push("(d.owner_id = ? OR d.owner_id = ?)".to_string());
                    query_params.push(owner.to_string().into());
                    query_params.push(guest_user_id.into());
                }
            } else {
                // No owner provided, default to only showing guest content.
                conditions.push("d.owner_id = ?".to_string());
                query_params.push(guest_user_id.into());
            }
        }
        #[cfg(not(feature = "core-access"))]
        {
            if let Some(owner) = owner_id {
                conditions.push("d.owner_id = ?".to_string());
                query_params.push(owner.to_string().into());
            } else {
                conditions.push("d.owner_id IS NULL".to_string());
            }
        }

        if let Some(ids) = document_ids {
            if !ids.is_empty() {
                let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                let condition = format!("de.document_id IN ({placeholders})");
                conditions.push(condition);
                for id in ids {
                    query_params.push(id.clone().into());
                }
            } else {
                return Ok(Vec::new());
            }
        }

        sql.push_str(&format!(" WHERE {}", conditions.join(" AND ")));
        sql.push_str(&format!(" ORDER BY similarity DESC LIMIT {limit};"));

        info!(sql = %sql, params = ?query_params, "Executing vector search SQL");

        let mut results = if query_params.is_empty() {
            conn.query(&sql, ()).await?
        } else {
            conn.query(&sql, query_params).await?
        };
        let mut search_results = Vec::new();

        while let Some(row) = results.next().await? {
            let title = match row.get_value(0)? {
                TursoValue::Text(s) => s,
                _ => String::new(),
            };
            let link = match row.get_value(1)? {
                TursoValue::Text(s) => s,
                _ => String::new(),
            };
            let content = match row.get_value(2)? {
                TursoValue::Text(s) => s,
                _ => String::new(),
            };
            let score = match row.get_value(3)? {
                TursoValue::Real(f) => f,
                _ => 0.0,
            };
            search_results.push(SearchResult {
                title,
                link,
                description: content,
                score,
            });
        }

        Ok(search_results)
    }
}

#[async_trait]
impl KeywordSearch for SqliteProvider {
    async fn keyword_search(
        &self,
        query: &str,
        limit: u32,
        owner_id: Option<&str>,
    ) -> Result<Vec<SearchResult>, SearchError> {
        info!("Executing keyword search for: '{query}' for owner: {owner_id:?}");
        let conn = self.db.connect()?;
        let pattern = format!("%{}%", query.to_lowercase());

        let mut conditions = vec!["(lower(content) LIKE ? OR lower(title) LIKE ?)".to_string()];
        let mut params = vec![TursoValue::Text(pattern.clone()), TursoValue::Text(pattern)];

        #[cfg(feature = "core-access")]
        {
            let guest_user_id =
                Uuid::new_v5(&Uuid::NAMESPACE_URL, GUEST_USER_IDENTIFIER.as_bytes()).to_string();
            if let Some(owner) = owner_id {
                if owner == guest_user_id {
                    // Guest user sees only guest content.
                    conditions.push("owner_id = ?".to_string());
                    params.push(TursoValue::Text(guest_user_id));
                } else {
                    // Authenticated user sees their own and guest content.
                    conditions.push("(owner_id = ? OR owner_id = ?)".to_string());
                    params.push(TursoValue::Text(owner.to_string()));
                    params.push(TursoValue::Text(guest_user_id));
                }
            } else {
                // No owner means guest-only view.
                conditions.push("owner_id = ?".to_string());
                params.push(TursoValue::Text(guest_user_id));
            }
        }
        #[cfg(not(feature = "core-access"))]
        {
            if let Some(owner) = owner_id {
                conditions.push("owner_id = ?".to_string());
                params.push(TursoValue::Text(owner.to_string()));
            } else {
                conditions.push("owner_id IS NULL".to_string());
            }
        }

        let sql = format!(
            "SELECT title, source_url, content FROM documents WHERE {} LIMIT {}",
            conditions.join(" AND "),
            limit
        );

        let mut results = conn.query(&sql, params).await?;
        let mut search_results = Vec::new();

        while let Some(row) = results.next().await? {
            let title = match row.get_value(0)? {
                TursoValue::Text(s) => s,
                _ => String::new(),
            };
            let link = match row.get_value(1)? {
                TursoValue::Text(s) => s,
                _ => String::new(),
            };
            let content = match row.get_value(2)? {
                TursoValue::Text(s) => s,
                _ => String::new(),
            };
            search_results.push(SearchResult {
                title,
                link,
                description: content,
                score: 0.5, // Using a default score for keyword matches
            });
        }

        Ok(search_results)
    }
}

#[async_trait]
impl MetadataSearch for SqliteProvider {
    async fn metadata_search(
        &self,
        entities: &[String],
        keyphrases: &[String],
        owner_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<String>, SearchError> {
        info!("Executing metadata search for entities: {entities:?}, keyphrases: {keyphrases:?}");
        let conn = self.db.connect()?;

        let mut conditions = Vec::new();
        let mut params: Vec<turso::Value> = Vec::new();

        #[cfg(feature = "core-access")]
        {
            let guest_user_id =
                Uuid::new_v5(&Uuid::NAMESPACE_URL, GUEST_USER_IDENTIFIER.as_bytes()).to_string();

            if let Some(owner) = owner_id {
                if owner == guest_user_id {
                    conditions.push("owner_id = ?".to_string());
                    params.push(guest_user_id.into());
                } else {
                    conditions.push("(owner_id = ? OR owner_id = ?)".to_string());
                    params.push(owner.to_string().into());
                    params.push(guest_user_id.into());
                }
            } else {
                conditions.push("owner_id = ?".to_string());
                params.push(guest_user_id.into());
            }
        }
        #[cfg(not(feature = "core-access"))]
        {
            if let Some(owner) = owner_id {
                conditions.push("owner_id = ?".to_string());
                params.push(owner.to_string().into());
            } else {
                conditions.push("owner_id IS NULL".to_string());
            }
        }

        let mut metadata_conditions = Vec::new();
        if !entities.is_empty() {
            let entity_likes: Vec<String> = entities
                .iter()
                .map(|_| "lower(metadata_value) LIKE ?".to_string())
                .collect();
            metadata_conditions.push(format!(
                "(metadata_type = 'ENTITY' AND ({}))",
                entity_likes.join(" OR ")
            ));
            for entity in entities {
                params.push(format!("%{}%", entity.to_lowercase()).into());
            }
        }
        if !keyphrases.is_empty() {
            let keyphrase_likes: Vec<String> = keyphrases
                .iter()
                .map(|_| "lower(metadata_value) LIKE ?".to_string())
                .collect();
            metadata_conditions.push(format!(
                "((metadata_type = 'KEYPHRASE' OR metadata_type = 'KEYPHRASES') AND ({}))",
                keyphrase_likes.join(" OR ")
            ));
            for keyphrase in keyphrases {
                params.push(format!("%{}%", keyphrase.to_lowercase()).into());
            }
        }

        if !metadata_conditions.is_empty() {
            conditions.push(format!("({})", metadata_conditions.join(" OR ")));
        } else {
            return Ok(Vec::new());
        }

        let where_clause = if conditions.is_empty() {
            "".to_string()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            "SELECT document_id, COUNT(document_id) as relevance_score
             FROM content_metadata
             {where_clause}
             GROUP BY document_id
             ORDER BY relevance_score DESC
             LIMIT {limit}"
        );

        let mut results = conn.query(&sql, params).await?;
        let mut doc_ids = Vec::new();

        while let Some(row) = results.next().await? {
            if let Ok(TursoValue::Text(id)) = row.get_value(0) {
                doc_ids.push(id);
            }
        }

        Ok(doc_ids)
    }
}
