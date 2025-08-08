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
use regex::Regex;
use tracing::{debug, error, info};

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

        let alias_instruction = match answer_key {
            Some(key) => format!(
                "If the query uses an aggregate function or returns a single column, alias the result with `AS {key}`."
            ),
            None => "If the query uses an aggregate function or returns a single column, choose a descriptive, single-word, lowercase alias for the result based on the user's question (e.g., for 'how many users', use `count`; for 'who is the manager', use `manager`).".to_string(),
        };

        let system_prompt = format!(
            "You are a {db_name} SQL expert. Write a readonly SQL query that answers the user's question. Expected output is a single SQL query only.",
            db_name = self.storage_provider.name()
        );

        let user_prompt = if !context.is_empty() {
            format!(
                "Follow these rules to create production-grade SQL:\n\
                1. For questions about \"who\", \"what\", or \"list\", use DISTINCT to avoid duplicate results.\n\
                2. When filtering, always explicitly exclude NULL values (e.g., `your_column IS NOT NULL`).\n\
                3. For date filtering, prefer using `EXTRACT(YEAR FROM your_column)` over functions like `FORMAT_TIMESTAMP`.\n\n\
                {alias_instruction}\n\n\
                Use the provided table schema to ensure the query is correct. Do not use placeholders for table or column names.\n\n\
                # Context\n{context}\n\n# User question\n{prompt}",
            )
        } else {
            prompt.to_string()
        };

        debug!(system_prompt = %system_prompt, user_prompt = %user_prompt, "--> Sending prompts to AI Provider");

        let raw_response = self
            .ai_provider
            .generate(&system_prompt, &user_prompt)
            .await?;

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

        let system_prompt = "You are a data transformation engine. Your only purpose is to transform the #INPUT data into the format described in the #OUTPUT section, based on the original #PROMPT. Do not add any explanations, summaries, or text that is not directly derived from the input data. Filter out any rows that are not relevant to the user's question.";
        let user_prompt = format!(
            r##"# PROMPT:
{prompt}

# OUTPUT:
{instruction}

# INPUT:
{content}
"##,
        );

        debug!(system_prompt = %system_prompt, user_prompt = %user_prompt, "--> Sending prompts to AI Provider for formatting");

        self.ai_provider.generate(system_prompt, &user_prompt).await
    }
}
