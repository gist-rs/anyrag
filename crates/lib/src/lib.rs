//! # Natural Language to BigQuery SQL
//!
//! This crate provides a client to convert natural language prompts into BigQuery SQL queries
//! using the Google Gemini API and execute them against a BigQuery project.

pub mod errors;
pub mod types;

pub use errors::PromptError;
pub use types::{PromptClient, PromptClientBuilder};

use gcp_bigquery_client::model::{
    query_request::QueryRequest, query_response::ResultSet, table::Table, table_schema::TableSchema,
};
use log::{debug, error, info};
use regex::Regex;
use std::sync::Arc;
use types::{Content, GeminiRequest, GeminiResponse, Part};

impl PromptClient {
    /// Executes a natural language prompt.
    ///
    /// This function sends the prompt to the Gemini API to get a SQL query,
    /// then executes the query on BigQuery and returns the result.
    pub async fn execute_prompt(
        &self,
        prompt: &str,
        table_name: Option<&str>,
    ) -> Result<String, PromptError> {
        let sql_query = self.get_sql_from_prompt(prompt, table_name).await?;

        if sql_query.trim().is_empty() {
            return Ok("The prompt did not result in a valid SQL query.".to_string());
        }

        let result = self.execute_bigquery_sql(&sql_query).await;
        if let Err(e) = &result {
            error!("[execute_prompt] Error: {:?}", e);
        }
        result
    }

    /// Converts a natural language prompt to a SQL query using the Gemini API.
    async fn get_sql_from_prompt(
        &self,
        prompt: &str,
        table_name: Option<&str>,
    ) -> Result<String, PromptError> {
        info!("[get_sql_from_prompt] received prompt: {:?}", prompt);
        let mut context = String::new();

        if let Some(table) = table_name {
            let schema = self.get_table_schema(table).await?;
            let schema_str = Self::format_schema_for_prompt(&schema);
            context.push_str(&format!("Schema for `{}`: ({}). ", table, schema_str));
        }

        let final_prompt = if !context.is_empty() {
            format!(
                "You are a BigQuery SQL expert. Write a readonly SQL query that answers the user's question. Use the provided table schema to ensure the query is correct. Do not use placeholders for table or column names. Expected output is a single SQL query only.\n\n# Context\n{}\n\n# User question\n{}",
                context, prompt
            )
        } else {
            prompt.to_string()
        };

        debug!("--> Prompt to Gemini: {}", &final_prompt);
        let request_body = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part { text: final_prompt }],
            }],
        };

        let response = self
            .gemini_client
            .post(&self.gemini_url)
            .query(&[("key", &self.gemini_api_key)])
            .json(&request_body)
            .send()
            .await
            .map_err(PromptError::GeminiRequest)?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(PromptError::GeminiApi(error_text));
        }

        let gemini_response: GeminiResponse = response
            .json()
            .await
            .map_err(PromptError::GeminiDeserialization)?;

        let raw_response = gemini_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .unwrap_or_default();

        debug!("<-- SQL from Gemini: {}", &raw_response);

        // Regex to extract SQL from markdown code blocks.
        let re = Regex::new(r"```(?:sql\n)?([\s\S]*?)```")?;
        let mut sql_query = re
            .captures(&raw_response)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_else(|| raw_response.trim().to_string());

        if let Some(table) = table_name {
            sql_query = sql_query.replace("`your_table_name`", &format!("`{}`", table));
            sql_query = sql_query.replace("your_table_name", table);
        }

        if !sql_query.trim().to_uppercase().starts_with("SELECT")
            && !sql_query.trim().to_uppercase().starts_with("WITH")
        {
            return Ok(String::new());
        }

        Ok(sql_query)
    }

    async fn get_table_schema(&self, table_name: &str) -> Result<Arc<TableSchema>, PromptError> {
        if let Some(schema) = self.schema_cache.read().await.get(table_name) {
            return Ok(schema.clone());
        }

        let parts: Vec<&str> = table_name.split('.').collect();
        if parts.len() != 3 {
            return Err(PromptError::BigQueryExecution(format!(
                "Invalid table name format: {}",
                table_name
            )));
        }
        let project_id = parts[0];
        let dataset_id = parts[1];
        let table_id = parts[2];

        let table: Table = self
            .bigquery_client
            .table()
            .get(project_id, dataset_id, table_id, None)
            .await?;

        let schema = table.schema;

        let schema_arc = Arc::new(schema);
        self.schema_cache
            .write()
            .await
            .insert(table_name.to_string(), schema_arc.clone());
        Ok(schema_arc)
    }

    fn format_schema_for_prompt(schema: &TableSchema) -> String {
        if let Some(fields) = &schema.fields {
            fields
                .iter()
                .map(|field| format!("{} {:?}", field.name, field.r#type))
                .collect::<Vec<String>>()
                .join(", ")
        } else {
            "".to_string()
        }
    }

    /// Executes a SQL query on BigQuery.
    async fn execute_bigquery_sql(&self, sql_query: &str) -> Result<String, PromptError> {
        info!("--> Executing BigQuery SQL: {}", sql_query);
        let response = self
            .bigquery_client
            .job()
            .query(
                &self.project_id,
                QueryRequest {
                    query: sql_query.to_string(),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| PromptError::BigQueryExecution(e.to_string()))?;

        let mut results = ResultSet::new_from_query_response(response);

        let mut result_string = String::new();
        let column_names = results.column_names();
        while results.next_row() {
            for name in &column_names {
                let value = results.get_json_value_by_name(name).unwrap();
                result_string.push_str(&format!("{}: {:?}, ", name, value));
            }
            result_string.push('\n');
        }

        Ok(result_string)
    }
}
