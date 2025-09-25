//! # Solana Transaction Generation Logic
//!
//! This module contains the core logic for generating Solana transactions
//! from natural language prompts using an AI provider.

use crate::types::{SolanaError, SolanaTransactionRequest, SolanaTransactionResponse};
use anyrag::providers::ai::AiProvider;
use tracing::{debug, warn};

/// The system prompt that instructs the LLM on how to generate the transaction.
const SYSTEM_PROMPT: &str = r#"You are an expert Solana transaction generator. Your task is to generate a raw Solana instruction in JSON format based on the user's request and the provided on-chain context.

# Rules
1.  You MUST respond with ONLY a valid JSON object.
2.  The JSON object MUST have three keys: `program_id`, `accounts`, and `data`.
3.  The `accounts` key must be an array of objects, each with `pubkey`, `is_signer`, and `is_writable` keys.
4.  The `data` key must be a string containing the Base58 encoded instruction data.
5.  You MUST use the information from the #CONTEXT and #USER_REQUEST to construct the transaction.
6.  Do not include any explanations, apologies, or markdown code fences in your response."#;

/// Cleans the raw JSON response from an LLM, removing markdown code fences.
fn clean_llm_response(response: &str) -> &str {
    response
        .trim()
        .strip_prefix("```json")
        .unwrap_or(response)
        .strip_suffix("```")
        .unwrap_or(response)
        .trim()
}

/// A struct that orchestrates the generation of Solana transactions.
pub struct TransactionGenerator<'a> {
    ai_provider: &'a dyn AiProvider,
}

impl<'a> TransactionGenerator<'a> {
    /// Creates a new `TransactionGenerator`.
    pub fn new(ai_provider: &'a dyn AiProvider) -> Self {
        Self { ai_provider }
    }

    /// Generates a Solana transaction from a natural language request.
    pub async fn generate(
        &self,
        request: &SolanaTransactionRequest<'_>,
    ) -> Result<SolanaTransactionResponse, SolanaError> {
        let user_prompt = format!(
            "#GENERATION_TASK\n{generation_prompt}\n\n#CONTEXT\n{context_prompt}\n\n#USER_REQUEST\n{prompt}",
            generation_prompt = request.generation_prompt,
            context_prompt = request.context_prompt,
            prompt = request.prompt
        );

        debug!("--> Sending prompts to AI for Solana transaction generation.");
        let llm_response = self
            .ai_provider
            .generate(SYSTEM_PROMPT, &user_prompt)
            .await?;
        debug!("<-- Raw response from AI: {}", llm_response);

        let cleaned_response = clean_llm_response(&llm_response);

        let parsed_response: SolanaTransactionResponse = serde_json::from_str(cleaned_response)
            .map_err(|e| {
                warn!(
                    "Failed to parse JSON from LLM, error: {}, raw response: '{}'",
                    e, cleaned_response
                );
                SolanaError::JsonParse(e)
            })?;

        Ok(parsed_response)
    }
}
