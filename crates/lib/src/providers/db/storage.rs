use crate::errors::PromptError;
use async_trait::async_trait;
use dyn_clone::DynClone;
use gcp_bigquery_client::model::table_schema::TableSchema;
use std::fmt::Debug;
use std::sync::Arc;

/// A trait for interacting with a storage backend.
///
/// This trait defines a common interface for executing queries and retrieving
/// schema information from different database providers (e.g., BigQuery, SQLite).
#[async_trait]
pub trait Storage: Send + Sync + DynClone + Debug {
    /// Returns the name of the storage provider (e.g., "BigQuery", "SQLite").
    fn name(&self) -> &str;

    /// Executes a SQL query against the storage provider.
    ///
    /// The result should be a JSON formatted string.
    async fn execute_sql(&self, sql: &str) -> Result<String, PromptError>;

    /// Retrieves the schema for a given table.
    async fn get_table_schema(&self, table_name: &str) -> Result<Arc<TableSchema>, PromptError>;
}

dyn_clone::clone_trait_object!(Storage);
