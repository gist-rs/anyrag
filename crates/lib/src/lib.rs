//! # Natural Language to Query
//!
//! This crate provides a client to convert natural language prompts into executable queries
//! (e.g., SQL) using a configurable AI provider and execute them against a storage provider.

#[cfg(feature = "graph_db")]
pub mod graph;

pub mod errors;
pub mod ingest;
pub mod prompts;
pub mod providers;
pub mod rerank;
pub mod search;
pub mod types;

pub use errors::PromptError;
pub use rerank::{RerankError, Rerankable};
pub use search::{SearchError, SearchMode};
pub use types::{
    ExecutePromptOptions, PromptClient, PromptClientBuilder, PromptResult, SearchResult,
};

use crate::prompts::{
    core::{get_alias_instruction, get_select_instruction, SQLITE_QUERY_USER_PROMPT},
    tasks::{
        QUERY_GENERATION_SYSTEM_PROMPT, QUERY_GENERATION_USER_PROMPT,
        RESPONSE_FORMATTING_SYSTEM_PROMPT, RESPONSE_FORMATTING_USER_PROMPT,
    },
};
use crate::types::TableSchema;
use chrono::Utc;
use serde_json::Value;
use tracing::{error, info};

/// Represents the result of a prompt that could be either a query or a direct answer.
pub enum QueryOrAnswer {
    Query(String),
    Answer(String),
}

impl PromptClient {
    /// Executes a natural language prompt with detailed options.
    ///
    /// This is the primary method for executing prompts. It follows the full
    /// "Text-to-Query" pipeline:
    ///
    /// 1.  It calls the AI provider to generate a query from the user's prompt and context.
    ///     This step can be customized with `system_prompt_template` and `user_prompt_template`.
    /// 2.  It executes the generated query against the configured storage provider.
    /// 3.  It optionally calls the AI provider again to format the raw query results into a
    ///     natural language response, guided by the `instruction`.
    pub async fn execute_prompt_with_options(
        &self,
        options: ExecutePromptOptions,
    ) -> Result<PromptResult, PromptError> {
        info!("[execute_prompt] Starting query generation pipeline.");
        let query_or_answer = self.get_query_from_prompt_internal(&options).await?;

        match query_or_answer {
            QueryOrAnswer::Query(query) => {
                if query.trim().is_empty() {
                    return Ok(PromptResult {
                        text: "The prompt did not result in a valid query.".to_string(),
                        ..Default::default()
                    });
                }

                let database_result = self.storage_provider.execute_query(&query).await;
                if let Err(e) = &database_result {
                    error!("[execute_prompt] Query execution error: {e:?}");
                }
                let database_result = database_result?;

                // Pre-process the JSON to make it more readable for the model.
                let json_data: serde_json::Value = serde_json::from_str(&database_result)?;
                let pretty_json = serde_json::to_string_pretty(&json_data)?;
                let final_result = self.format_response(&pretty_json, &options).await?;

                Ok(PromptResult {
                    text: final_result,
                    generated_sql: Some(query),
                    database_result: Some(database_result),
                })
            }
            QueryOrAnswer::Answer(answer) => {
                if answer.trim().is_empty() {
                    return Ok(PromptResult {
                        text: "The prompt did not result in a valid query.".to_string(),
                        ..Default::default()
                    });
                }
                Ok(PromptResult {
                    text: answer,
                    ..Default::default()
                })
            }
        }
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
    ) -> Result<PromptResult, PromptError> {
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
    pub async fn execute_prompt_from_value(
        &self,
        value: Value,
    ) -> Result<PromptResult, PromptError> {
        let options: ExecutePromptOptions = serde_json::from_value(value)?;
        self.execute_prompt_with_options(options).await
    }

    /// Converts a natural language prompt to a query using the configured AI provider.
    /// This is public to allow for testing the query generation step in isolation.
    pub async fn get_query_from_prompt(
        &self,
        options: &ExecutePromptOptions,
    ) -> Result<PromptResult, PromptError> {
        match self.get_query_from_prompt_internal(options).await? {
            QueryOrAnswer::Query(q) => Ok(PromptResult {
                text: q,
                ..Default::default()
            }),
            // For backward compatibility and simple testing, return empty string for non-queries.
            QueryOrAnswer::Answer(_) => Ok(PromptResult::default()),
        }
    }

    /// Internal version of `get_query_from_prompt` that distinguishes between queries and direct answers.
    async fn get_query_from_prompt_internal(
        &self,
        options: &ExecutePromptOptions,
    ) -> Result<QueryOrAnswer, PromptError> {
        info!(
            "[get_query_from_prompt] received prompt: {:?}",
            options.prompt
        );

        let now = Utc::now();
        let today_rfc2822 = now.to_rfc2822();
        let today_iso8601 = now.to_rfc3339();

        let mut context = format!("# TODAY\nRFC2822: {today_rfc2822}\nUTC: {today_iso8601}\n\n");
        let language = self.storage_provider.language();

        let alias_instruction = get_alias_instruction(options.answer_key.as_deref());
        let select_instruction = get_select_instruction(options.instruction.as_deref());

        // If a content_type is provided, we use specialized prompts.
        // Otherwise, we fall back to the table-based or direct question logic.
        let (system_prompt, user_prompt) = if let Some(content_type) = &options.content_type {
            // --- Logic for ContentType-based prompts ---
            info!("[get_query_from_prompt] Using ContentType-based prompt generation: {content_type:?}");
            // Always start with the base context (date) and append specific content if available.
            let final_context = if let Some(content) = &options.context {
                if content.is_empty() {
                    context.clone()
                } else {
                    // The base context already ends with \n\n, so we just append.
                    format!("{context}{content}")
                }
            } else {
                context.clone()
            };

            let (system_template, user_template) = content_type.get_prompt_templates();

            // Allow overriding the ContentType default with a specific template from options
            let system_prompt = options
                .system_prompt_template
                .clone()
                .unwrap_or_else(|| system_template.to_string())
                .replace("{language}", language)
                .replace("{db_name}", self.storage_provider.name());

            let user_prompt = options
                .user_prompt_template
                .clone()
                .unwrap_or_else(|| user_template.to_string())
                .replace("{context}", &final_context)
                .replace("{prompt}", &options.prompt)
                .replace("{language}", language)
                .replace("{alias_instruction}", &alias_instruction);

            (system_prompt, user_prompt)
        } else if options.table_name.is_some() {
            // --- Logic for Query Generation ---
            info!("[get_query_from_prompt] Using table-based query generation.");

            // If a specific table is named, get its schema.
            if let Some(table) = &options.table_name {
                let schema = self.storage_provider.get_table_schema(table).await?;
                let schema_str = Self::format_schema_for_prompt(&schema);
                context.push_str(&format!("# Schema for `{table}`\n{schema_str}\n\n"));
            } else {
                // If no specific table is named, but a DB is context, get all table schemas.
                info!("[get_query_from_prompt] No table_name provided; fetching all schemas for the current DB.");
                let tables = self.storage_provider.list_tables().await?;
                for table in tables {
                    // It's possible for schema fetching to fail for a specific table.
                    // We'll log the error but continue, so the AI gets as much context as possible.
                    match self.storage_provider.get_table_schema(&table).await {
                        Ok(schema) => {
                            let schema_str = Self::format_schema_for_prompt(&schema);
                            context.push_str(&format!("# Schema for `{table}`\n{schema_str}\n\n"));
                        }
                        Err(e) => {
                            error!(
                                "[get_query_from_prompt] Failed to get schema for table '{}': {}",
                                table, e
                            );
                        }
                    }
                }
            }

            let system_prompt = options.system_prompt_template.clone().unwrap_or_else(|| {
                QUERY_GENERATION_SYSTEM_PROMPT
                    .replace("{language}", language)
                    .replace("{db_name}", self.storage_provider.name())
            });

            let user_prompt = if let Some(template) = &options.user_prompt_template {
                template
                    .replace("{language}", language)
                    .replace("{context}", &context)
                    .replace("{prompt}", &options.prompt)
                    .replace("{select_instruction}", &select_instruction)
                    .replace("{alias_instruction}", &alias_instruction)
            } else if self.storage_provider.name() == "SQLite" {
                // If the storage provider is SQLite, use the specialized user prompt.
                SQLITE_QUERY_USER_PROMPT
                    .replace("{language}", language)
                    .replace("{context}", &context)
                    .replace("{prompt}", &options.prompt)
                    .replace("{select_instruction}", &select_instruction)
                    .replace("{alias_instruction}", &alias_instruction)
            } else {
                QUERY_GENERATION_USER_PROMPT
                    .replace("{language}", language)
                    .replace("{context}", &context)
                    .replace("{prompt}", &options.prompt)
                    .replace("{select_instruction}", &select_instruction)
                    .replace("{alias_instruction}", &alias_instruction)
            };
            (system_prompt, user_prompt)
        } else {
            // --- Logic for Direct Questions ---
            info!("[get_query_from_prompt] Using direct question mode.");
            let system_prompt = options
                .system_prompt_template
                .clone()
                .unwrap_or_else(|| "You are a helpful AI assistant.".to_string());

            let user_prompt = if let Some(template) = &options.user_prompt_template {
                template
                    .replace("{context}", &context)
                    .replace("{prompt}", &options.prompt)
                    // The other placeholders don't apply here.
                    .replace("{language}", language)
                    .replace("{alias_instruction}", &alias_instruction)
            } else {
                // For a direct question, the prompt is just context + question.
                format!(
                    "# Context\n{}\n\n# User question\n{}",
                    context.trim_end(),
                    options.prompt
                )
            };
            (system_prompt, user_prompt)
        };

        info!(system_prompt = %system_prompt, user_prompt = %user_prompt, "--> Sending prompts to AI Provider");

        let raw_response = self
            .ai_provider
            .generate(&system_prompt, &user_prompt)
            .await?;

        info!("<-- Raw response from AI: {}", &raw_response);

        // --- FINAL, ROBUST LOGIC ---
        // This logic robustly handles AI responses that are either raw SQL
        // or SQL inside a markdown code block.
        let trimmed_response = raw_response.trim();
        let query_candidate =
            if trimmed_response.starts_with("```") && trimmed_response.ends_with("```") {
                // It's a markdown block. Slice to get the content inside.
                let mut inner_content = &trimmed_response[3..trimmed_response.len() - 3];

                // The first line might be the language specifier (e.g., "sql\n").
                // If so, trim it off.
                if let Some(newline_pos) = inner_content.find('\n') {
                    let first_line = &inner_content[..newline_pos].trim();
                    if !first_line.contains(' ') {
                        // A simple language specifier won't have spaces.
                        inner_content = &inner_content[newline_pos + 1..];
                    }
                }
                inner_content.trim()
            } else {
                // Not a markdown block, treat the whole response as the candidate.
                trimmed_response
            };

        // Now, check if the candidate (which is now clean SQL) looks like a query.
        if !query_candidate.to_uppercase().starts_with("SELECT")
            && !query_candidate.to_uppercase().starts_with("WITH")
        {
            // If not, it's a direct answer. The answer is the *original* raw response.
            info!("[get_query_from_prompt] Response is a direct answer, not a query.");
            return Ok(QueryOrAnswer::Answer(raw_response.to_string()));
        }

        info!("[get_query_from_prompt] Successfully generated query.");

        // It is a query, so perform table name replacements on the cleaned candidate.
        let mut query = query_candidate.to_string();
        if let Some(table) = &options.table_name {
            query = query.replace("`your_table_name`", &format!("`{table}`"));
            query = query.replace("your_table_name", table);
        }

        Ok(QueryOrAnswer::Query(query))
    }

    /// Formats a `TableSchema` into a markdown-like string for the AI prompt.
    fn format_schema_for_prompt(schema: &TableSchema) -> String {
        schema
            .fields
            .iter()
            .map(|field| {
                let mut field_str = format!(
                    "- {field_name}: {field_type:?}",
                    field_name = field.name,
                    field_type = field.r#type
                );
                if let Some(desc) = &field.description {
                    if !desc.is_empty() {
                        field_str.push_str(&format!(" ({desc})"));
                    }
                }
                field_str
            })
            .collect::<Vec<String>>()
            .join("\n")
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
            .unwrap_or_else(|| RESPONSE_FORMATTING_SYSTEM_PROMPT.to_string());

        let user_prompt = if let Some(template) = &options.format_user_prompt_template {
            template
                .replace("{prompt}", &options.prompt)
                .replace("{instruction}", instruction)
                .replace("{content}", content)
        } else {
            RESPONSE_FORMATTING_USER_PROMPT
                .replace("{prompt}", &options.prompt)
                .replace("{instruction}", instruction)
                .replace("{content}", content)
        };

        info!(system_prompt = %system_prompt, user_prompt = %user_prompt, "--> Sending prompts to AI Provider for formatting");

        self.ai_provider
            .generate(&system_prompt, &user_prompt)
            .await
    }
}
