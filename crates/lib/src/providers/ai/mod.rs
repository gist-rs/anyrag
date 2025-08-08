pub mod gemini;
pub mod local;

use crate::errors::PromptError;
use async_trait::async_trait;
use dyn_clone::DynClone;
use std::fmt::Debug;

/// A trait for interacting with an AI provider.
///
/// This trait defines a common interface for generating SQL queries from natural language
/// using different Large Language Models (e.g., Gemini, local models).
#[async_trait]
pub trait AiProvider: Send + Sync + Debug + DynClone {
    /// Generates a SQL query from a given prompt.
    ///
    /// The result should be a string containing a valid SQL query.
    async fn generate_sql(&self, prompt: &str) -> Result<String, PromptError>;
}

dyn_clone::clone_trait_object!(AiProvider);
