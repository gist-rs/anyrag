//! # Admin Route Handlers
//!
//! This module contains handlers for endpoints that require administrative (root) privileges.

use crate::{
    auth::middleware::AuthenticatedUser,
    errors::AppError,
    handlers::{wrap_response, ApiResponse, DebugParams},
    state::AppState,
};
use axum::{
    extract::{Query, State},
    Json,
};
use serde::Serialize;
use serde_json::json;
use tracing::info;

/// A response item for the user list.
#[derive(Serialize)]
pub struct UserListResponse {
    id: String,
    role: String,
    created_at: String,
}

/// Handler for retrieving a list of all users.
///
/// **Authorization**: This endpoint is protected and only accessible by users with the 'root' role.
pub async fn get_users_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
) -> Result<Json<ApiResponse<Vec<UserListResponse>>>, AppError> {
    let current_user = user.0;
    info!(
        "User '{}' with role '{}' is attempting to access the all users list.",
        current_user.id, current_user.role
    );

    // --- Authorization Check ---
    if current_user.role != "root" {
        // This is not ideal as it will result in a 500 status code, but it's the
        // only suitable error variant available right now. This should be improved
        // by adding an `AppError::Forbidden` variant.
        return Err(AppError::Internal(anyhow::anyhow!(
            "Forbidden: You do not have permission to access this resource."
        )));
    }

    let conn = app_state.sqlite_provider.db.connect()?;
    let mut rows = conn
        .query(
            "SELECT id, role, created_at FROM users ORDER BY created_at DESC",
            (),
        )
        .await?;

    let mut users = Vec::new();
    while let Some(row) = rows.next().await? {
        users.push(UserListResponse {
            id: row.get(0)?,
            role: row.get(1)?,
            created_at: row.get(2)?,
        });
    }

    let debug_info = json!({ "requesting_user_id": current_user.id, "user_count": users.len() });
    Ok(wrap_response(users, debug_params, Some(debug_info)))
}
