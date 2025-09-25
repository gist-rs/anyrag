use serde::{Deserialize, Serialize};
use thiserror::Error;

// --- Error Definition ---

#[derive(Error, Debug)]
pub enum SolanaError {
    #[error("LLM processing failed: {0}")]
    Llm(#[from] anyrag::PromptError),
    #[error("Failed to parse LLM response as JSON: {0}")]
    JsonParse(#[from] serde_json::Error),
}

// --- Data Structures ---

/// Represents the input required to generate a Solana transaction.
#[derive(Debug, Serialize)]
pub struct SolanaTransactionRequest<'a> {
    /// The on-chain context (e.g., account states, key maps).
    pub context_prompt: &'a str,
    /// The specific instruction for the LLM.
    pub generation_prompt: &'a str,
    /// The user's natural language request.
    pub prompt: &'a str,
}

/// Represents a single account in a Solana transaction's instruction.
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct AccountMeta {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

/// Represents the structured JSON output for a raw Solana transaction.
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct SolanaTransactionResponse {
    pub program_id: String,
    pub accounts: Vec<AccountMeta>,
    pub data: String, // Base64 or Base58 encoded instruction data
}
