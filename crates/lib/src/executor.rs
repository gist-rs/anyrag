//! # The Core Executor
//!
//! This module defines the `AnyragExecutor`, which is the primary entry point for
//! executing the core business logic of the application. It holds all necessary
//! dependencies, such as AI and storage providers, and exposes high-level methods
//! that can be called by any consumer (like the `server` or `cli` crates).

use crate::{
    constants,
    ingest::{ingest_from_google_sheet_url, sheet_url_to_export_url_and_table_name},
    providers::{
        ai::AiProvider,
        db::{sqlite::SqliteProvider, storage::Storage},
        factory::create_dynamic_provider,
    },
    types::{
        AppConfig, ContentType, ExecutePromptOptions as LibExecutePromptOptions,
        HttpRequestPromptOptions, ResolvedTask,
    },
    PromptClientBuilder, PromptError, PromptResult,
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

/// A struct that holds all the dependencies required to execute `anyrag`'s core logic.
/// This decouples the business logic from the server's `AppState` or any other
/// specific application container.
pub struct AnyragExecutor {
    pub ai_providers: Arc<HashMap<String, Box<dyn AiProvider>>>,
    pub sqlite_provider: Arc<SqliteProvider>,
    pub config: Arc<AppConfig>,
    pub tasks: Arc<HashMap<String, ResolvedTask>>,
}

impl AnyragExecutor {
    /// Creates a new `AnyragExecutor`.
    pub fn new(
        ai_providers: Arc<HashMap<String, Box<dyn AiProvider>>>,
        sqlite_provider: Arc<SqliteProvider>,
        config: Arc<AppConfig>,
        tasks: Arc<HashMap<String, ResolvedTask>>,
    ) -> Self {
        Self {
            ai_providers,
            sqlite_provider,
            config,
            tasks,
        }
    }

    /// Orchestrates the execution of a prompt originating from an HTTP request.
    /// This is the primary entry point for the `server` crate into the `lib`'s core logic.
    /// It encapsulates business logic such as shorthand command parsing, dynamic provider
    /// selection, and on-the-fly ingestion.
    pub async fn execute_http_prompt(
        &self,
        mut options: HttpRequestPromptOptions,
    ) -> Result<PromptResult, PromptError> {
        info!("Executor received prompt payload: '{}'", options.prompt);

        // --- Shorthand "ls" command: Always targets a local DB ---
        if options.prompt.starts_with("ls ") {
            info!("Shorthand 'ls' command detected. Overriding to local DB query.");
            let parts: Vec<&str> = options.prompt.split_whitespace().collect();
            let table_name = match parts.get(1) {
                Some(tn) => tn.to_string(),
                None => {
                    return Err(PromptError::StorageOperationFailed(
                        "'ls' command requires a table name.".to_string(),
                    ));
                }
            };

            let mut limit = 10; // Default limit
            if let Some(limit_part) = parts.get(2) {
                if let Some(limit_str) = limit_part.strip_prefix("limit=") {
                    limit = limit_str.parse().unwrap_or(10);
                }
            }

            // This shorthand requires a db context.
            let _db_name = options
                .db
                .as_deref()
                .or(options.project_id.as_deref())
                .ok_or_else(|| {
                    PromptError::StorageOperationFailed(
                        "'ls' command requires a 'db' or 'project_id' field.".to_string(),
                    )
                })?;

            options.table_name = Some(table_name.clone());
            options.prompt = format!(
                "List the first {limit} rows from the `{table_name}` table, showing all columns."
            );
            info!("Transformed prompt: '{}'", options.prompt);
        }

        // --- On-the-fly Google Sheet Ingestion ---
        let sheet_url = options
            .prompt
            .split_whitespace()
            .find(|word| word.contains("/spreadsheets/d/"));

        if let Some(url) = sheet_url {
            // This logic requires the concrete SqliteProvider, which we have in the executor.
            let (export_url, table_name) =
                sheet_url_to_export_url_and_table_name(url).map_err(|e| {
                    PromptError::StorageOperationFailed(format!(
                        "Sheet URL transformation failed: {e}"
                    ))
                })?;

            if self
                .sqlite_provider
                .get_table_schema(&table_name)
                .await
                .is_err()
            {
                info!("Table '{}' does not exist. Starting ingestion.", table_name);
                ingest_from_google_sheet_url(&self.sqlite_provider.db, &export_url, &table_name)
                    .await
                    .map_err(|e| {
                        PromptError::StorageOperationFailed(format!("Sheet ingestion failed: {e}"))
                    })?;
            } else {
                info!("Table '{}' already exists. Skipping ingestion.", table_name);
            }
            // Modify the options for the rest of the pipeline.
            options.table_name = Some(table_name);
        }

        // --- Task-based Configuration Loading ---
        let task_name = match options.content_type {
            #[cfg(feature = "rss")]
            Some(ContentType::Rss) => "rss_summarization",
            Some(ContentType::Knowledge) => "rag_synthesis",
            _ => {
                if options.table_name.is_some()
                    || options.project_id.is_some()
                    || options.db.is_some()
                {
                    "query_generation"
                } else {
                    "direct_generation"
                }
            }
        };
        info!("Selected task '{task_name}' based on request payload.");

        let task_config = self.tasks.get(task_name).ok_or_else(|| {
            PromptError::StorageOperationFailed(format!(
                "Configuration for task '{task_name}' not found."
            ))
        })?;

        // --- AI Provider Selection ---
        let (ai_provider, _model_used_name) = if let Some(model_name) = &options.model {
            create_dynamic_provider(&self.config.providers, model_name)?
        } else {
            let provider_name = &task_config.provider;
            let provider = self
                .ai_providers
                .get(provider_name)
                .ok_or_else(|| {
                    PromptError::MissingAiProvider(format!(
                    "Provider '{provider_name}' for task '{task_name}' not found in providers map."
                ))
                })?
                .clone();
            let provider_config = self.config.providers.get(provider_name).unwrap();
            (provider, provider_config.model_name.clone())
        };

        // Apply task's default prompts if not overridden in the request.
        if options.system_prompt_template.is_none() {
            options.system_prompt_template = Some(task_config.system_prompt.clone());
        }
        if options.user_prompt_template.is_none() {
            options.user_prompt_template = Some(task_config.user_prompt.clone());
        }

        // --- Storage Provider Selection ---
        let storage_provider: Box<dyn Storage> = if let Some(_project_id) =
            options.project_id.as_deref()
        {
            info!("'project_id' provided. Creating a dynamic BigQuery client for this request.");
            #[cfg(feature = "bigquery")]
            {
                let bq_provider =
                    crate::providers::db::bigquery::BigQueryProvider::new(_project_id.to_string())
                        .await?;
                Box::new(bq_provider)
            }
            #[cfg(not(feature = "bigquery"))]
            {
                return Err(crate::PromptError::BigQueryFeatureNotEnabled);
            }
        } else if let Some(db_name) = options.db.as_deref() {
            info!(
                "'db' provided: '{}'. Creating a dynamic SQLite client for this request.",
                db_name
            );
            let db_path = format!("{}/{db_name}.db", constants::DB_DIR);
            let provider = SqliteProvider::new(&db_path).await?;
            provider.initialize_schema().await?;
            Box::new(provider)
        } else {
            // Default to the main SQLite provider from the executor.
            info!("No 'project_id' or 'db'. Using default SQLite provider.");
            Box::new(self.sqlite_provider.as_ref().clone())
        };

        // --- Final Execution ---
        let client = PromptClientBuilder::new()
            .ai_provider(ai_provider)
            .storage_provider(storage_provider)
            .build()?;

        let lib_options: LibExecutePromptOptions = options.into();
        client.execute_prompt_with_options(lib_options).await
    }
}
