//! # Natural Language to Query
//!
//! This crate provides a client to convert natural language prompts into executable queries
//! (e.g., SQL) using a configurable AI provider and execute them against a storage provider.

pub mod errors;
pub mod prompts;
pub mod providers;
pub mod types;

pub use errors::PromptError;
pub use types::{ExecutePromptOptions, PromptClient, PromptClientBuilder};

use crate::prompts::{
    DEFAULT_FORMAT_SYSTEM_PROMPT, DEFAULT_FORMAT_USER_PROMPT, DEFAULT_QUERY_SYSTEM_PROMPT,
    DEFAULT_QUERY_USER_PROMPT,
};
use gcp_bigquery_client::model::table_schema::TableSchema;
use regex::Regex;
use serde_json::Value;
use tracing::{debug, error, info};

impl PromptClient {
    /// Executes a natural language prompt with detailed options.
    ///
    /// This is the primary method for executing prompts. It supports two modes:
    ///
    /// 1.  **Query Generation (Default):** If `system_prompt_template` is `None`, it generates a
    ///     query from the prompt, executes it against the storage provider, and formats the response.
    /// 2.  **Generic Prompting:** If a `system_prompt_template` is provided, it bypasses all query
    ///     generation and execution logic. It directly sends the system and user prompts to the
    ///     AI provider and returns the raw response. This is useful for tasks like translation
    ///     or summarization.
    pub async fn execute_prompt_with_options(
        &self,
        options: ExecutePromptOptions,
    ) -> Result<String, PromptError> {
        // If a custom system prompt for the main task is provided, switch to generic mode.
        if let Some(system_prompt) = options.system_prompt_template.clone() {
            info!("[execute_prompt] Generic mode: custom system prompt provided.");
            // In this mode, we just send the prompts to the AI and return the response directly.
            let user_prompt = &options.prompt;
            return self.ai_provider.generate(&system_prompt, user_prompt).await;
        }

        // --- Default Mode: Query Generation & Execution ---
        info!("[execute_prompt] Query generation mode.");
        let query = self.get_query_from_prompt(&options).await?;

        if query.trim().is_empty() {
            return Ok("The prompt did not result in a valid query.".to_string());
        }

        let result = self.storage_provider.execute_query(&query).await;
        if let Err(e) = &result {
            error!("[execute_prompt] Query execution error: {e:?}");
        }
        let result = result?;

        // Pre-process the JSON to make it more readable for the model.
        let json_data: serde_json::Value = serde_json::from_str(&result)?;
        let pretty_json = serde_json::to_string_pretty(&json_data)?;
        self.format_response(&pretty_json, &options).await
    }

    /// Executes a natural language prompt with basic parameters.
    ///
    /// This is a convenience wrapper around `execute_prompt_with_options` for backward compatibility
    /// and simpler use cases.
    pub async fn execute_prompt(
        &self,
        prompt: &str,
        table_name: Option<&str>,
        instruction: Option<&str>,
        answer_key: Option<&str>,
    ) -> Result<String, PromptError> {
        let options = ExecutePromptOptions {
            prompt: prompt.to_string(),
            table_name: table_name.map(String::from),
            instruction: instruction.map(String::from),
            answer_key: answer_key.map(String::from),
            ..Default::default()
        };
        self.execute_prompt_with_options(options).await
    }

    /// Executes a prompt from a serde_json::Value.
    ///
    /// This allows for easy integration with APIs that receive JSON payloads.
    pub async fn execute_prompt_from_value(&self, value: Value) -> Result<String, PromptError> {
        let options: ExecutePromptOptions = serde_json::from_value(value)?;
        self.execute_prompt_with_options(options).await
    }

    /// Converts a natural language prompt to a query using the configured AI provider.
    /// This is public to allow for testing the query generation step in isolation.
    pub async fn get_query_from_prompt(
        &self,
        options: &ExecutePromptOptions,
    ) -> Result<String, PromptError> {
        info!(
            "[get_query_from_prompt] received prompt: {:?}",
            options.prompt
        );
        let mut context = String::new();
        let language = self.storage_provider.language();

        if let Some(table) = &options.table_name {
            let schema = self.storage_provider.get_table_schema(table).await?;
            let schema_str = Self::format_schema_for_prompt(&schema);
            context.push_str(&format!("Schema for `{table}`: ({schema_str}). "));
        }

        let alias_instruction = match &options.answer_key {
            Some(key) => format!(
                "If the query uses an aggregate function or returns a single column, alias the result with `AS {key}`."
            ),
            None => "If the query uses an aggregate function or returns a single column, choose a descriptive, single-word, lowercase alias for the result based on the user's question (e.g., for 'how many users', use `count`; for 'who is the manager', use `manager`).".to_string(),
        };

        // This is the default system prompt for query generation.
        let system_prompt = options.system_prompt_template.clone().unwrap_or_else(|| {
            DEFAULT_QUERY_SYSTEM_PROMPT
                .replace("{language}", language)
                .replace("{db_name}", self.storage_provider.name())
        });

        let user_prompt = if let Some(template) = &options.user_prompt_template {
            template
                .replace("{language}", language)
                .replace("{context}", &context)
                .replace("{prompt}", &options.prompt)
                .replace("{alias_instruction}", &alias_instruction)
        } else if !context.is_empty() {
            DEFAULT_QUERY_USER_PROMPT
                .replace("{language}", language)
                .replace("{context}", &context)
                .replace("{prompt}", &options.prompt)
                .replace("{alias_instruction}", &alias_instruction)
        } else {
            options.prompt.to_string()
        };

        debug!(system_prompt = %system_prompt, user_prompt = %user_prompt, "--> Sending prompts to AI Provider");

        let raw_response = self
            .ai_provider
            .generate(&system_prompt, &user_prompt)
            .await?;

        debug!("<-- Query from AI: {}", &raw_response);

        // Regex to extract a query from markdown code blocks.
        let re = Regex::new(r"```(?:sql|query)?\n?([\s\S]*?)```")?;
        let mut query = re
            .captures(&raw_response)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_else(|| raw_response.trim().to_string());

        if let Some(table) = &options.table_name {
            query = query.replace("`your_table_name`", &format!("`{table}`"));
            query = query.replace("your_table_name", table);
        }

        // Note: This is a simple validation that works for SQL-like languages.
        if !query.trim().to_uppercase().starts_with("SELECT")
            && !query.trim().to_uppercase().starts_with("WITH")
        {
            return Ok(String::new());
        }

        Ok(query)
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
        options: &ExecutePromptOptions,
    ) -> Result<String, PromptError> {
        let instruction = match &options.instruction {
            Some(inst) => inst,
            None => return Ok(content.to_string()),
        };

        info!("[format_response] received instruction: {instruction:?}");

        let system_prompt = options
            .format_system_prompt_template
            .clone()
            .unwrap_or_else(|| DEFAULT_FORMAT_SYSTEM_PROMPT.to_string());

        let user_prompt = if let Some(template) = &options.format_user_prompt_template {
            template
                .replace("{prompt}", &options.prompt)
                .replace("{instruction}", instruction)
                .replace("{content}", content)
        } else {
            DEFAULT_FORMAT_USER_PROMPT
                .replace("{prompt}", &options.prompt)
                .replace("{instruction}", instruction)
                .replace("{content}", content)
        };

        debug!(system_prompt = %system_prompt, user_prompt = %user_prompt, "--> Sending prompts to AI Provider for formatting");

        self.ai_provider
            .generate(&system_prompt, &user_prompt)
            .await
    }
}
