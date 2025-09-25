//! # API Route Handlers
//!
//! This module organizes all the Axum route handlers for the `anyrag-server`.
//! The handlers are split into logical sub-modules based on their functionality
//! (e.g., `ingest`, `search`, `knowledge`).

// Sub-modules for different handler categories.
pub mod admin_handlers;
pub mod auth_handlers;
pub mod db_handlers;
pub mod document_handlers;
pub mod general;
pub mod generation_handlers;
#[cfg(feature = "solana")]
pub mod generation_transaction_handlers;
pub mod generation_types;
pub mod graph_handlers;
pub mod ingest;
pub mod knowledge;
pub mod search;

// Re-export all handlers from the sub-modules to make them easily accessible
// to the router under a single `handlers::` path.
pub use admin_handlers::*;
pub use auth_handlers::*;
pub use db_handlers::*;
pub use document_handlers::*;
pub use general::*;
pub use generation_handlers::*;
#[cfg(feature = "solana")]
pub use generation_transaction_handlers::*;
pub use graph_handlers::*;
pub use ingest::*;
pub use knowledge::*;
pub use search::*;

// Shared items used by multiple handler modules.
use super::{
    errors::AppError,
    state::AppState,
    types::{ApiResponse, DebugParams},
};
use axum::{extract::Query, Json};
use serde_json::Value;

/// A shared helper function to wrap a successful result in the standard `ApiResponse`
/// format, optionally including debug information if requested.
pub(crate) fn wrap_response<T>(
    result: T,
    debug_params: Query<DebugParams>,
    debug_info: Option<Value>,
) -> Json<ApiResponse<T>> {
    let debug = if debug_params.debug.unwrap_or(false) {
        debug_info
    } else {
        None
    };
    Json(ApiResponse { debug, result })
}
