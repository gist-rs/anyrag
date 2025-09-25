use super::{wrap_response, ApiResponse, AppError, AppState, DebugParams, PromptResponse};
use crate::auth::middleware::AuthenticatedUser;
use anyrag_solana::{SolanaTransactionRequest, TransactionGenerator};
use axum::{
    extract::{Query, State},
    Json,
};

use super::generation_types::GenTextRequest;
use anyhow::anyhow;
use serde_json::json;
use tracing::info;

pub async fn gen_tx_handler(
    State(app_state): State<AppState>,
    _user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<GenTextRequest>,
) -> Result<Json<ApiResponse<PromptResponse>>, AppError> {
    info!("Handling /gen/tx request");

    // For now, we'll just use the default provider.
    // In the future, this could be selected based on the user's request or config.
    let ai_provider = app_state
        .ai_providers
        .get("local_default")
        .ok_or_else(|| AppError::Internal(anyhow!("Provider 'local_default' not found")))?;

    let generator = TransactionGenerator::new(ai_provider.as_ref());

    // The GenTextRequest is reused here. We map its `generation_prompt` field
    // to the main user 'prompt' for the Solana generator. The 'generation_prompt'
    // for the Solana generator is a more static instruction.
    let request = SolanaTransactionRequest {
        context_prompt: payload.context_prompt.as_deref().unwrap_or(""),
        generation_prompt: "Your task is to generate a raw Solana instruction in JSON format based on the user's request and the provided on-chain context. Your response must be a JSON object with `program_id`, `accounts`, and `data` keys.",
        prompt: &payload.generation_prompt,
    };

    let transaction = generator
        .generate(&request)
        .await
        .map_err(|e| AppError::Internal(anyhow!(e)))?;

    let final_value = serde_json::to_value(transaction)?;

    // For now, debug info is not implemented for this handler.
    let debug_info = json!({});

    Ok(wrap_response(
        PromptResponse { text: final_value },
        debug_params,
        Some(debug_info),
    ))
}
