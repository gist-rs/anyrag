use crate::types::{FieldType as AnyragFieldType, TableField, TableSchema as AnyragTableSchema};
use crate::{errors::PromptError, providers::db::storage::Storage};
use async_trait::async_trait;
use gcp_bigquery_client::{
    model::{
        field_type::FieldType as BqFieldType, query_request::QueryRequest,
        query_response::ResultSet, table::Table, table_schema::TableSchema as BqTableSchema,
    },
    Client,
};
use serde_json::Value;
use std::{
    collections::HashMap,
    fmt::{self, Debug},
    sync::Arc,
};
use tokio::sync::RwLock;
use tracing::debug;

/// A provider for interacting with Google BigQuery.
#[derive(Clone)]
pub struct BigQueryProvider {
    client: Client,
    project_id: String,
    schema_cache: Arc<RwLock<HashMap<String, Arc<AnyragTableSchema>>>>,
}

impl BigQueryProvider {
    /// Creates a new `BigQueryProvider`.
    pub async fn new(project_id: String) -> Result<Self, PromptError> {
        let client = Client::from_application_default_credentials()
            .await
            .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?;
        Ok(Self {
            client,
            project_id,
            schema_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Converts a BigQuery-specific schema to the provider-agnostic `AnyragTableSchema`.
    fn convert_schema(bq_schema: &BqTableSchema) -> AnyragTableSchema {
        let fields = bq_schema
            .fields
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|bq_field| TableField {
                name: bq_field.name.clone(),
                r#type: match bq_field.r#type {
                    BqFieldType::String => AnyragFieldType::String,
                    BqFieldType::Integer => AnyragFieldType::Integer,
                    BqFieldType::Float => AnyragFieldType::Float,
                    BqFieldType::Boolean => AnyragFieldType::Boolean,
                    BqFieldType::Timestamp => AnyragFieldType::Timestamp,
                    BqFieldType::Date => AnyragFieldType::Date,
                    BqFieldType::Bytes => AnyragFieldType::Bytes,
                    BqFieldType::Json => AnyragFieldType::Json,
                    // Default to String for complex/unhandled types like GEOGRAPHY, etc.
                    _ => AnyragFieldType::String,
                },
                description: bq_field.description.clone(),
            })
            .collect();

        AnyragTableSchema { fields }
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
    fn name(&self) -> &str {
        "BigQuery"
    }

    fn language(&self) -> &str {
        "SQL"
    }

    /// Executes a query on BigQuery and returns the result as a JSON string.
    async fn execute_query(&self, query: &str) -> Result<String, PromptError> {
        debug!(query = %query, "--> Executing BigQuery query");

        // The query job is always run in the provider's configured project,
        // which has the necessary billing and permissions. The query string itself
        // (e.g., "SELECT * FROM `bigquery-public-data.samples.shakespeare`")
        // tells BigQuery which project to read the data from.
        // By explicitly setting `use_legacy_sql` to false, we ensure Standard SQL
        // is used, which is generally required for modern queries and syntax. This can
        // also prevent the client from making incorrect assumptions about default datasets.
        let mut req = QueryRequest::new(query.to_string());
        req.use_legacy_sql = false;

        let response = self
            .client
            .job()
            .query(&self.project_id, req)
            .await
            .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?;

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
    async fn get_table_schema(
        &self,
        table_name: &str,
    ) -> Result<Arc<AnyragTableSchema>, PromptError> {
        if let Some(schema) = self.schema_cache.read().await.get(table_name) {
            return Ok(schema.clone());
        }

        let parts: Vec<&str> = table_name.split('.').collect();
        if parts.len() != 3 {
            return Err(PromptError::StorageOperationFailed(format!(
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
            .map_err(|e| PromptError::StorageOperationFailed(e.to_string()))?;

        // Convert the BQ-specific schema to our generic, provider-agnostic schema.
        let bq_schema = table.schema;
        let anyrag_schema = Self::convert_schema(&bq_schema);
        let schema_arc = Arc::new(anyrag_schema);

        self.schema_cache
            .write()
            .await
            .insert(table_name.to_string(), schema_arc.clone());

        Ok(schema_arc)
    }

    /// Lists all tables in the BigQuery project.
    /// Note: This is a placeholder implementation.
    async fn list_tables(&self) -> Result<Vec<String>, PromptError> {
        // This is a complex operation in BigQuery, requiring iterating through datasets.
        // For now, we return an empty list as it's not critical for the SQLite-focused feature.
        Ok(Vec::new())
    }
}
