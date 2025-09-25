//! # Solana Transaction Generation
//!
//! This crate provides a `TransactionGenerator` that uses an AI provider
//! to convert natural language prompts into raw Solana transaction JSON.

use anyhow::Result;
use anyrag::providers::ai::AiProvider;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, warn};

// --- Error Definition ---

#[derive(Error, Debug)]
pub enum SolanaError {
    #[error("LLM processing failed: {0}")]
    Llm(#[from] anyrag::PromptError),
    #[error("Failed to parse LLM response as JSON: {0}")]
    JsonParse(#[from] serde_json::Error),
}

// --- Data Structures ---

#[derive(Debug, Serialize)]
pub struct SolanaTransactionRequest<'a> {
    /// The on-chain context (e.g., account states, key maps).
    pub context_prompt: &'a str,
    /// The specific instruction for the LLM.
    pub generation_prompt: &'a str,
    /// The user's natural language request.
    pub prompt: &'a str,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct AccountMeta {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct SolanaTransactionResponse {
    pub program_id: String,
    pub accounts: Vec<AccountMeta>,
    pub data: String, // Base64 or Base58 encoded instruction data
}

// --- Core Logic ---

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

        let parsed_response: SolanaTransactionResponse =
            serde_json::from_str(cleaned_response).map_err(|e| {
                warn!(
                    "Failed to parse JSON from LLM, error: {}, raw response: '{}'",
                    e, cleaned_response
                );
                SolanaError::JsonParse(e)
            })?;

        Ok(parsed_response)
    }
}

// --- Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use anyrag::providers::ai::local::LocalAiProvider;
    use serde_json::json;
    use std::sync::{Arc, RwLock};

    // A simple mock AI provider for testing purposes.
    #[derive(Clone, Debug)]
    pub struct MockAiProvider {
        pub call_history: Arc<RwLock<Vec<(String, String)>>>,
        pub responses: Arc<RwLock<Vec<String>>>,
    }

    impl MockAiProvider {
        pub fn new(responses: Vec<String>) -> Self {
            Self {
                call_history: Arc::new(RwLock::new(Vec::new())),
                responses: Arc::new(RwLock::new(responses.into_iter().rev().collect())),
            }
        }
    }

    #[async_trait::async_trait]
    impl AiProvider for MockAiProvider {
        async fn generate(
            &self,
            system_prompt: &str,
            user_prompt: &str,
        ) -> Result<String, anyrag::PromptError> {
            self.call_history
                .write()
                .unwrap()
                .push((system_prompt.to_string(), user_prompt.to_string()));

            if let Some(response) = self.responses.write().unwrap().pop() {
                Ok(response)
            } else {
                Ok("{}".to_string()) // Default empty JSON
            }
        }
    }

    #[tokio::test]
    async fn test_generate_transaction_successfully() {
        // Arrange
        let expected_response = SolanaTransactionResponse {
            program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
            accounts: vec![
                AccountMeta {
                    pubkey: "6nrJ4TdMSMz4omJQA6R5c3TDnfQ1UoBJ1ux7UGsB2pcv".to_string(),
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: "7aVgJrZvZ6wTayTR3CVYPqLCNBGw1pB5aUbaqx6RijYX".to_string(),
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: "3i7ijk5nAZwWzKvduAehYXJDu9SnLanEKyrtr9Ru382E".to_string(),
                    is_signer: true,
                    is_writable: false,
                },
            ],
            data: "3kVA21YASy2b".to_string(),
        };
        let mock_response_json = serde_json::to_string(&expected_response).unwrap();
        let mock_ai_provider = MockAiProvider::new(vec![mock_response_json]);
        let call_history = mock_ai_provider.call_history.clone();

        let generator = TransactionGenerator::new(&mock_ai_provider);
        let request = SolanaTransactionRequest {
            context_prompt: "---...---",
            generation_prompt: "Your task is...",
            prompt: "Please send 15 USDC...",
        };

        // Act
        let result = generator.generate(&request).await.unwrap();

        // Assert
        assert_eq!(result, expected_response);

        let history = call_history.read().unwrap();
        assert_eq!(history.len(), 1);
        let (system_prompt, user_prompt) = &history[0];
        assert_eq!(system_prompt, SYSTEM_PROMPT);
        assert!(user_prompt.contains(request.context_prompt));
        assert!(user_prompt.contains(request.generation_prompt));
        assert!(user_prompt.contains(request.prompt));
    }
}
