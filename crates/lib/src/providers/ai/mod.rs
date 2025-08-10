pub mod embedding;
pub mod gemini;
pub mod local;

use crate::errors::PromptError;
use async_trait::async_trait;
use dyn_clone::DynClone;
pub use embedding::generate_embedding;
use std::fmt::Debug;

/// A trait for interacting with an AI provider.
///
/// This trait defines a common interface for generating SQL queries from natural language
/// using different Large Language Models (e.g., Gemini, local models).
#[async_trait]
pub trait AiProvider: Send + Sync + Debug + DynClone {
    /// Generates a response from a given system and user prompt.
    ///
    /// The result should be a string containing the AI's response.
    async fn generate(&self, system_prompt: &str, user_prompt: &str)
        -> Result<String, PromptError>;
}

dyn_clone::clone_trait_object!(AiProvider);
