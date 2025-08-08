use crate::{errors::PromptError, providers::db::storage::Storage};
use async_trait::async_trait;
use gcp_bigquery_client::{
    model::{
        query_request::QueryRequest, query_response::ResultSet, table::Table,
        table_schema::TableSchema,
    },
    Client,
};
use log::info;
use serde_json::Value;
use std::{
    collections::HashMap,
    fmt::{self, Debug},
    sync::Arc,
};
use tokio::sync::RwLock;

/// A provider for interacting with Google BigQuery.
#[derive(Clone)]
pub struct BigQueryProvider {
    client: Client,
    project_id: String,
    schema_cache: Arc<RwLock<HashMap<String, Arc<TableSchema>>>>,
}

impl BigQueryProvider {
    /// Creates a new `BigQueryProvider`.
    pub async fn new(project_id: String) -> Result<Self, PromptError> {
        let client = Client::from_application_default_credentials()
            .await
            .map_err(|e| PromptError::StorageQueryFailed(e.to_string()))?;
        Ok(Self {
            client,
            project_id,
            schema_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}

impl Debug for BigQueryProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BigQueryProvider")
            .field("project_id", &self.project_id)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl Storage for BigQueryProvider {
    /// Executes a SQL query on BigQuery and returns the result as a JSON string.
    async fn execute_sql(&self, sql_query: &str) -> Result<String, PromptError> {
        info!("--> Executing BigQuery SQL: {sql_query}");
        let response = self
            .client
            .job()
            .query(
                &self.project_id,
                QueryRequest {
                    query: sql_query.to_string(),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| PromptError::StorageQueryFailed(e.to_string()))?;

        let mut results = ResultSet::new_from_query_response(response);
        let mut json_results: Vec<Value> = Vec::new();
        let column_names = results.column_names();

        while results.next_row() {
            let mut row_map = serde_json::Map::new();
            for name in &column_names {
                let value = results
                    .get_json_value_by_name(name)
                    .ok()
                    .flatten()
                    .unwrap_or(Value::Null);
                row_map.insert(name.clone(), value);
            }
            json_results.push(Value::Object(row_map));
        }

        Ok(serde_json::to_string(&json_results)?)
    }

    /// Retrieves the schema for a given BigQuery table.
    async fn get_table_schema(&self, table_name: &str) -> Result<Arc<TableSchema>, PromptError> {
        if let Some(schema) = self.schema_cache.read().await.get(table_name) {
            return Ok(schema.clone());
        }

        let parts: Vec<&str> = table_name.split('.').collect();
        if parts.len() != 3 {
            return Err(PromptError::StorageQueryFailed(format!(
                "Invalid table name format for BigQuery: {table_name}. Expected format: project.dataset.table"
            )));
        }
        let project_id = parts[0];
        let dataset_id = parts[1];
        let table_id = parts[2];

        let table: Table = self
            .client
            .table()
            .get(project_id, dataset_id, table_id, None)
            .await
            .map_err(|e| PromptError::StorageQueryFailed(e.to_string()))?;

        let schema = table.schema;
        let schema_arc = Arc::new(schema);
        self.schema_cache
            .write()
            .await
            .insert(table_name.to_string(), schema_arc.clone());

        Ok(schema_arc)
    }
}
