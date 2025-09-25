use anyrag::providers::ai::AiProvider;
use anyrag_solana::{
    generator::TransactionGenerator,
    types::{AccountMeta, SolanaTransactionRequest, SolanaTransactionResponse},
};

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
async fn test_generate_transaction_successfully_1() {
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
            context_prompt: "---\n\nCURRENT ON-CHAIN CONTEXT:\naccount_states:\n  RECIPIENT_USDC_ATA:\n    lamports: 2039280\n  USER_USDC_ATA:\n    lamports: 2039280\n  USER_WALLET_PUBKEY:\n    lamports: 1000000000\nkey_map:\n  RECIPIENT_USDC_ATA: 7aVgJrZvZ6wTayTR3CVYPqLCNBGw1pB5aUbaqx6RijYX\n  USER_USDC_ATA: 6nrJ4TdMSMz4omJQA6R5c3TDnfQ1UoBJ1ux7UGsB2pcv\n  USER_WALLET_PUBKEY: 3i7ijk5nAZwWzKvduAehYXJDu9SnLanEKyrtr9Ru382E\n\n\n---",
            generation_prompt: "Your task is to generate a raw Solana instruction in JSON format based on the user's request and the provided on-chain context. Your response must be a JSON object with `program_id`, `accounts`, and `data` keys.",
            prompt: "Please send 15 USDC from my token account (USER_USDC_ATA) to the recipient's token account (RECIPIENT_USDC_ATA). The mint is MOCK_USDC_MINT, and I am the authority (USER_WALLET_PUBKEY).",
        };

    // Act
    let result = generator.generate(&request).await.unwrap();

    // Assert
    assert_eq!(result, expected_response);

    let history = call_history.read().unwrap();
    assert_eq!(history.len(), 1);
    let (_system_prompt, user_prompt) = &history[0];
    assert!(user_prompt.contains(request.context_prompt));
    assert!(user_prompt.contains(request.generation_prompt));
    assert!(user_prompt.contains(request.prompt));
}

#[tokio::test]
async fn test_generate_spl_token_transfer_successfully() {
    // Arrange
    let expected_response = SolanaTransactionResponse {
        program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
        accounts: vec![
            AccountMeta {
                pubkey: "6ncs6yoFTaGG7Ysn1jFVqC4ypUYPTtUuZRgKtDYa71Eg".to_string(),
                is_signer: false,
                is_writable: true,
            },
            AccountMeta {
                pubkey: "6fU7UgdcwRw3BaQ4PuN7mMJ2FPh4enk9NwArxfqpPcqa".to_string(),
                is_signer: false,
                is_writable: true,
            },
            AccountMeta {
                pubkey: "GfpQVWVYA24PcWrQchMNwEmYtLTUyW26Dhoq7JFfZhVs".to_string(),
                is_signer: true,
                is_writable: false,
            },
        ],
        data: "3kVA21YASy2b".to_string(),
    };
    let mock_response_json = serde_json::to_string(&expected_response).unwrap();
    let mock_ai_provider = MockAiProvider::new(vec![mock_response_json]);

    let generator = TransactionGenerator::new(&mock_ai_provider);
    let request = SolanaTransactionRequest {
        context_prompt: "---\n\nCURRENT ON-CHAIN CONTEXT:\naccount_states:\n  RECIPIENT_USDC_ATA:\n    lamports: 2039280\n  USER_USDC_ATA:\n    lamports: 2039280\n  USER_WALLET_PUBKEY:\n    lamports: 1000000000\nkey_map:\n  RECIPIENT_USDC_ATA: 6fU7UgdcwRw3BaQ4PuN7mMJ2FPh4enk9NwArxfqpPcqa\n  USER_USDC_ATA: 6ncs6yoFTaGG7Ysn1jFVqC4ypUYPTtUuZRgKtDYa71Eg\n  USER_WALLET_PUBKEY: GfpQVWVYA24PcWrQchMNwEmYtLTUyW26Dhoq7JFfZhVs\n\n\n---",
        generation_prompt: "Your task is to generate a raw Solana instruction in JSON format based on the user's request and the provided on-chain context. Your response must be a JSON object with `program_id`, `accounts`, and `data` keys.",
        prompt: "Please send 15 USDC from my token account (USER_USDC_ATA) to the recipient's token account (RECIPIENT_USDC_ATA). The mint is MOCK_USDC_MINT, and I am the authority (USER_WALLET_PUBKEY).",
    };

    // Act
    let result = generator.generate(&request).await.unwrap();

    // Assert
    assert_eq!(result, expected_response);
}
