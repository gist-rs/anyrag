//! # Document Route Handlers
//!
//! This module contains handlers for document-related endpoints.

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
use core_access::GUEST_USER_IDENTIFIER;
use serde::Serialize;
use serde_json::json;
use tracing::info;
use uuid::Uuid;

/// A response item for the document list.
#[derive(Serialize)]
pub struct DocumentListResponse {
    pub id: String,
    pub owner_id: String,
    pub source_url: String,
    pub title: String,
    pub created_at: String,
}

/// Handler for retrieving a list of documents.
///
/// **Authorization**: This endpoint is protected.
/// - Users with the 'root' role can see all documents.
/// - Regular users can see their own documents and documents owned by the guest user.
/// - Guest users can only see guest-owned documents.
pub async fn get_documents_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
) -> Result<Json<ApiResponse<Vec<DocumentListResponse>>>, AppError> {
    let current_user = user.0;
    info!(
        "User '{}' with role '{}' is fetching documents.",
        current_user.id, current_user.role
    );

    let conn = app_state.sqlite_provider.db.connect()?;
    let guest_user_id =
        Uuid::new_v5(&Uuid::NAMESPACE_URL, GUEST_USER_IDENTIFIER.as_bytes()).to_string();

    let (query_sql, params) = if current_user.role == "root" {
        (
            "SELECT id, owner_id, source_url, title, created_at FROM documents ORDER BY created_at DESC",
            vec![],
        )
    } else if current_user.id == guest_user_id {
        (
            "SELECT id, owner_id, source_url, title, created_at FROM documents WHERE owner_id = ? ORDER BY created_at DESC",
            vec![turso::Value::Text(guest_user_id)],
        )
    } else {
        (
            "SELECT id, owner_id, source_url, title, created_at FROM documents WHERE owner_id = ? OR owner_id = ? ORDER BY created_at DESC",
            vec![turso::Value::Text(current_user.id.clone()), turso::Value::Text(guest_user_id)],
        )
    };

    let mut rows = conn.query(query_sql, params).await?;
    let mut documents = Vec::new();
    while let Some(row) = rows.next().await? {
        documents.push(DocumentListResponse {
            id: row.get(0)?,
            owner_id: row.get(1).unwrap_or_default(),
            source_url: row.get(2).unwrap_or_default(),
            title: row.get(3).unwrap_or_default(),
            created_at: row.get(4).unwrap_or_default(),
        });
    }

    let debug_info =
        json!({ "requesting_user_id": current_user.id, "document_count": documents.len() });
    Ok(wrap_response(documents, debug_params, Some(debug_info)))
}
