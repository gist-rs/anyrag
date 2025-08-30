use crate::{
    errors::PromptError,
    providers::db::storage::{KeywordSearch, Storage, VectorSearch},
    search::SearchError,
    types::SearchResult,
};
use async_trait::async_trait;
use gcp_bigquery_client::model::{
    field_type::FieldType, table_field_schema::TableFieldSchema, table_schema::TableSchema,
};
use serde_json::Value;
use std::{
    collections::HashMap,
    fmt::{self, Debug},
    sync::Arc,
};
use tokio::sync::RwLock;
use tracing::{debug, info};
use turso::{params, Database, Value as TursoValue};

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

    /// Performs a vector similarity search on the `faq_kb` table.
    pub async fn vector_search_faqs(
        &self,
        query_vector: Vec<f32>,
        limit: u32,
    ) -> Result<Vec<FaqSearchResult>, SearchError> {
        info!("Executing SQLite vector search on faq_kb.");
        let conn = self.db.connect()?;

        let vector_str = format!(
            "vector('[{}]')",
            query_vector
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );

        // A similarity score of 0.65 means we are looking for vectors that are reasonably close.
        // This corresponds to a cosine distance of 0.7. (1.0 - (0.7 / 2.0) = 0.65)
        let similarity_threshold = 0.65;
        let distance_calculation =
            format!("(1.0 - (vector_distance_cos(embedding, {vector_str}) / 2.0))");
        let sql = format!(
            "SELECT answer, {distance_calculation} AS similarity
             FROM faq_kb
             WHERE embedding IS NOT NULL AND {distance_calculation} > {similarity_threshold}
             ORDER BY similarity DESC
             LIMIT {limit};"
        );

        let mut results = conn.query(&sql, ()).await?;
        let mut search_results = Vec::new();

        while let Some(row) = results.next().await? {
            let answer = match row.get_value(0)? {
                TursoValue::Text(s) => s,
                _ => String::new(),
            };
            let score = match row.get_value(1)? {
                TursoValue::Real(f) => f,
                _ => 0.0,
            };
            search_results.push(FaqSearchResult { answer, score });
        }

        Ok(search_results)
    }

    /// Performs a keyword search on the `faq_kb` table.
    pub async fn keyword_search_faqs(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<FaqSearchResult>, SearchError> {
        info!("Executing keyword search on faq_kb for: {}", query);
        let conn = self.db.connect()?;
        let pattern = format!("%{}%", query.to_lowercase());

        let sql = sql::keyword_search_faqs(limit);

        let mut results = conn.query(&sql, params![pattern]).await?;
        let mut search_results = Vec::new();

        while let Some(row) = results.next().await? {
            let answer = match row.get_value(0)? {
                TursoValue::Text(s) => s,
                _ => String::new(),
            };
            let score = match row.get_value(1)? {
                TursoValue::Real(f) => f,
                _ => 0.0,
            };
            search_results.push(FaqSearchResult { answer, score });
        }

        Ok(search_results)
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
                let bq_type = match type_str.to_uppercase().as_str() {
                    "INTEGER" => FieldType::Integer,
                    "TEXT" => FieldType::String,
                    "REAL" => FieldType::Float,
                    "BLOB" => FieldType::Bytes,
                    "DATETIME" | "TIMESTAMP" => FieldType::Timestamp,
                    "DATE" => FieldType::Date,
                    // NUMERIC and other types default to String for simplicity
                    _ => FieldType::String,
                };

                fields.push(TableFieldSchema {
                    name,
                    r#type: bq_type,
                    mode: Some("NULLABLE".to_string()),
                    fields: None,
                    description: None, // Keep it simple, no format hints.
                    categories: None,
                    policy_tags: None,
                });
            }
        }

        if fields.is_empty() {
            return Err(PromptError::StorageOperationFailed(format!(
                "Table '{table_name}' not found or has no columns."
            )));
        }

        info!(table_name = %table_name, "Successfully fetched schema with {} columns.", fields.len());

        let schema = Arc::new(TableSchema {
            fields: Some(fields),
        });

        self.schema_cache
            .write()
            .await
            .insert(table_name.to_string(), schema.clone());

        Ok(schema)
    }
}

#[async_trait]
impl VectorSearch for SqliteProvider {
    /// Performs a vector similarity search using SQLite with the vss-lite extension.
    async fn vector_search(
        &self,
        query_vector: Vec<f32>,
        limit: u32,
    ) -> Result<Vec<SearchResult>, SearchError> {
        info!("Executing SQLite vector search query.");
        let conn = self.db.connect()?;

        // The vector is formatted into a string that the `vector` function in SQLite can parse.
        let vector_str = format!(
            "vector('[{}]')",
            query_vector
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );

        // A similarity score of 0.7 means we are looking for vectors that are reasonably close.
        // This corresponds to a cosine distance of 0.6. (1.0 - (0.6 / 2.0) = 0.7)
        let similarity_threshold = 0.7;
        let distance_calculation =
            format!("(1.0 - (vector_distance_cos(embedding, {vector_str}) / 2.0))");
        let sql = format!(
            "SELECT title, link, description, {distance_calculation} AS similarity
             FROM articles
             WHERE embedding IS NOT NULL AND {distance_calculation} > {similarity_threshold}
             ORDER BY similarity DESC
             LIMIT {limit};"
        );

        let mut results = conn.query(&sql, ()).await?;
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
            let description = match row.get_value(2)? {
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
                description,
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
    ) -> Result<Vec<SearchResult>, SearchError> {
        info!("Executing LIKE keyword search query for: {}", query);
        let conn = self.db.connect()?;
        // Convert the query to lowercase for a case-insensitive search.
        let pattern = format!("%{}%", query.to_lowercase());

        let sql = sql::keyword_search_articles(limit);

        let mut results = conn.query(&sql, params![pattern]).await?;
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
            let description = match row.get_value(2)? {
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
                description,
                score,
            });
        }

        Ok(search_results)
    }
}
