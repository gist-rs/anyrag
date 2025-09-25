//! # Solana Transaction Generation
//!
//! This crate provides a `TransactionGenerator` that uses an AI provider
//! to convert natural language prompts into raw Solana transaction JSON.
//!
//! It is designed as a plugin for the `anyrag` ecosystem and follows the
//! architectural principles of separating concerns.
//!
//! ## Core Components
//!
//! -   **`TransactionGenerator`**: The main struct that orchestrates the AI call.
//! -   **`SolanaTransactionRequest`**: The input struct defining the prompt and context.
//! -   **`SolanaTransactionResponse`**: The structured JSON output representing the transaction.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use anyrag::providers::ai::local::LocalAiProvider;
//! use anyrag_solana::{TransactionGenerator, SolanaTransactionRequest};
//!
//! async fn generate_tx() {
//!     // Assume ai_provider is already initialized
//!     let ai_provider = LocalAiProvider::new("http://localhost:1234/v1/chat/completions".to_string(), None, None).unwrap();
//!     let generator = TransactionGenerator::new(&ai_provider);
//!
//!     let request = SolanaTransactionRequest {
//!         context_prompt: "...",
//!         generation_prompt: "...",
//!         prompt: "Send 15 USDC...",
//!     };
//!
//!     match generator.generate(&request).await {
//!         Ok(tx) => println!("Generated Program ID: {}", tx.program_id),
//!         Err(e) => eprintln!("Error: {}", e),
//!     }
//! }
//! ```

pub mod generator;
pub mod types;

pub use generator::TransactionGenerator;
pub use types::{AccountMeta, SolanaError, SolanaTransactionRequest, SolanaTransactionResponse};
