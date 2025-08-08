//! # Natural Language to BigQuery SQL
//!
//! This crate provides a client to convert natural language prompts into BigQuery SQL queries
//! using the Google Gemini API and execute them against a BigQuery project.

pub mod errors;
pub mod providers;
pub mod types;

pub use errors::PromptError;
pub use types::{PromptClient, PromptClientBuilder};

use gcp_bigquery_client::model::table_schema::TableSchema;
use log::{debug, error, info};
use regex::Regex;

impl PromptClient {
    /// Executes a natural language prompt.
    ///
    /// This function sends the prompt to the Gemini API to get a SQL query,
    /// then executes the query on BigQuery and returns the result.
    pub async fn execute_prompt(
        &self,
        prompt: &str,
        table_name: Option<&str>,
        instruction: Option<&str>,
        answer_key: Option<&str>,
    ) -> Result<String, PromptError> {
        let sql_query = self
            .get_sql_from_prompt(prompt, table_name, answer_key)
            .await?;

        if sql_query.trim().is_empty() {
            return Ok("The prompt did not result in a valid SQL query.".to_string());
        }

        let result = self.storage_provider.execute_sql(&sql_query).await;
        if let Err(e) = &result {
            error!("[execute_prompt] Error: {e:?}");
        }
        let result = result?;

        // Pre-process the JSON to make it more readable for the model.
        let json_data: serde_json::Value = serde_json::from_str(&result)?;
        let pretty_json = serde_json::to_string_pretty(&json_data)?;
        self.format_response(&pretty_json, prompt, instruction)
            .await
    }

    /// Converts a natural language prompt to a SQL query using the configured AI provider.
    async fn get_sql_from_prompt(
        &self,
        prompt: &str,
        table_name: Option<&str>,
        answer_key: Option<&str>,
    ) -> Result<String, PromptError> {
        info!("[get_sql_from_prompt] received prompt: {prompt:?}");
        let mut context = String::new();

        if let Some(table) = table_name {
            let schema = self.storage_provider.get_table_schema(table).await?;
            let schema_str = Self::format_schema_for_prompt(&schema);
            context.push_str(&format!("Schema for `{table}`: ({schema_str}). "));
        }

        let alias_instruction = format!(
            "If the query uses an aggregate function like COUNT, SUM, etc., alias the result with `AS {}`.",
            answer_key.unwrap_or("answer")
        );

        let final_prompt = if !context.is_empty() {
            format!(
                "You are a BigQuery SQL expert. Write a readonly SQL query that answers the user's question. {alias_instruction} Use the provided table schema to ensure the query is correct. Do not use placeholders for table or column names. Expected output is a single SQL query only.\n\n# Context\n{context}\n\n# User question\n{prompt}"
            )
        } else {
            prompt.to_string()
        };

        debug!("--> Prompt to AI Provider: {}", &final_prompt);

        let raw_response = self.ai_provider.generate_sql(&final_prompt).await?;

        debug!("<-- SQL from Gemini: {}", &raw_response);

        // Regex to extract SQL from markdown code blocks.
        // Regex to extract SQL from markdown code blocks, which many models use.
        let re = Regex::new(r"```(?:sql\n)?([\s\S]*?)```")?;
        let mut sql_query = re
            .captures(&raw_response)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_else(|| raw_response.trim().to_string());

        if let Some(table) = table_name {
            sql_query = sql_query.replace("`your_table_name`", &format!("`{table}`"));
            sql_query = sql_query.replace("your_table_name", table);
        }

        if !sql_query.trim().to_uppercase().starts_with("SELECT")
            && !sql_query.trim().to_uppercase().starts_with("WITH")
        {
            return Ok(String::new());
        }

        Ok(sql_query)
    }

    fn format_schema_for_prompt(schema: &TableSchema) -> String {
        if let Some(fields) = &schema.fields {
            fields
                .iter()
                .map(|field| {
                    format!(
                        "{field_name} {field_type:?}",
                        field_name = field.name,
                        field_type = field.r#type
                    )
                })
                .collect::<Vec<String>>()
                .join(", ")
        } else {
            "".to_string()
        }
    }

    /// Formats the raw query result using the AI provider if an instruction is given.
    async fn format_response(
        &self,
        content: &str,
        prompt: &str,
        instruction: Option<&str>,
    ) -> Result<String, PromptError> {
        let instruction = match instruction {
            Some(inst) => inst,
            None => return Ok(content.to_string()),
        };

        info!("[format_response] received instruction: {instruction:?}");

        let final_prompt = format!(
            r##"You are a data formatting engine. Your sole purpose is to transform the #INPUT data based on the #PROMPT and #OUTPUT instructions. Do not add any explanations, apologies, or extra text.

# PROMPT:
{prompt}

# OUTPUT:
{instruction}

# INPUT:
{content}
"##,
        );

        debug!(
            "--> Prompt to AI Provider for formatting: {}",
            &final_prompt
        );

        self.ai_provider.generate_sql(&final_prompt).await
    }
}
