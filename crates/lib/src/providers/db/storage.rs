use crate::{errors::PromptError, search::SearchError, types::SearchResult};
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

    /// Returns the query language used by the storage provider (e.g., "SQL").
    fn language(&self) -> &str;

    /// Executes a query against the storage provider.
    ///
    /// The result should be a JSON formatted string.
    async fn execute_query(&self, query: &str) -> Result<String, PromptError>;

    /// Retrieves the schema for a given table.
    async fn get_table_schema(&self, table_name: &str) -> Result<Arc<TableSchema>, PromptError>;
}

dyn_clone::clone_trait_object!(Storage);

/// A trait for providers that support vector similarity search.
#[async_trait]
pub trait VectorSearch: Send + Sync + DynClone + Debug {
    /// Performs a vector similarity search.
    async fn vector_search(
        &self,
        query_vector: Vec<f32>,
        limit: u32,
    ) -> Result<Vec<SearchResult>, SearchError>;
}

dyn_clone::clone_trait_object!(VectorSearch);

/// A trait for providers that support keyword search.
#[async_trait]
pub trait KeywordSearch: Send + Sync + DynClone + Debug {
    /// Performs a keyword search.
    async fn keyword_search(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<SearchResult>, SearchError>;
}

dyn_clone::clone_trait_object!(KeywordSearch);
