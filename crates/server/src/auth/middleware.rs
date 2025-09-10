//! # Authentication Middleware
//!
//! This module provides the Axum middleware for handling JWT-based authentication.
//! It defines an `AuthenticatedUser` extractor that can be used in handlers to
//! ensure a valid user is present and to get their identity.

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use chrono::Utc;
use core_access::{get_or_create_user, User, GUEST_USER_IDENTIFIER};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};

use crate::state::AppState;

/// Represents the claims we expect to find in the JWT.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// The subject of the token, which we use as the unique user identifier.
    pub sub: String,
    /// The expiration timestamp.
    pub exp: usize,
    /// The user's database ID (UUID). This is optional and mainly for testing.
    #[serde(default)]
    pub user_id: String,
}

/// An Axum extractor that provides the currently authenticated user.
///
/// This extractor implements the logic defined in `NOW.md`:
/// 1.  **No Token Present**: Resolves to a deterministic "Guest User".
/// 2.  **Valid Token Present**: Resolves to the authenticated user.
/// 3.  **Invalid/Expired Token Present**: Rejects the request with a `401 Unauthorized`.
///
/// This ensures that handlers always receive a valid `User` object (either
/// guest or authenticated), simplifying the application logic.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser(pub User);

/// A custom rejection type for authentication failures.
///
/// This allows the `FromRequestParts` implementation to return a specific
/// HTTP status code and error message, which Axum then turns into a response.
pub struct AuthError(StatusCode, String);

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Attempt to extract the token from the `Authorization: Bearer <token>` header.
        // This is now optional.
        let bearer_header =
            Option::<TypedHeader<Authorization<Bearer>>>::from_request_parts(parts, state)
                .await
                .map_err(|e| {
                    // This error should ideally not happen for an optional extractor unless something is malformed.
                    warn!("Unexpected error during header extraction: {}", e);
                    AuthError(
                        StatusCode::BAD_REQUEST,
                        "Invalid Authorization header format.".to_string(),
                    )
                })?;

        let user = if let Some(TypedHeader(Authorization(bearer))) = bearer_header {
            // Case 1: Token is present. Validate it.
            info!("Authorization header found, attempting to validate JWT.");
            let jwt_secret =
                std::env::var("JWT_SECRET").unwrap_or_else(|_| "a-secure-secret-key".to_string());

            let token_data = decode::<Claims>(
                bearer.token(),
                &DecodingKey::from_secret(jwt_secret.as_ref()),
                &Validation::default(),
            )
            .map_err(|e| {
                warn!("JWT validation failed: {}", e);
                AuthError(
                    StatusCode::UNAUTHORIZED,
                    "Invalid or expired token.".to_string(),
                )
            })?;

            // Manually verify the expiration to be absolutely sure.
            // The `jsonwebtoken` crate should handle this, but adding an explicit
            // check makes the logic more robust against subtle configuration issues.
            let current_timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| {
                    AuthError(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "System time is before UNIX EPOCH.".to_string(),
                    )
                })?
                .as_secs();

            if token_data.claims.exp < current_timestamp as usize {
                warn!(
                    "Token has expired. exp: {}, current: {}",
                    token_data.claims.exp, current_timestamp
                );
                return Err(AuthError(
                    StatusCode::UNAUTHORIZED,
                    "Invalid or expired token.".to_string(),
                ));
            }

            // If user_id is provided in the claim, construct the user directly.
            // This is primarily for testing scenarios to inject a specific user.
            if !token_data.claims.user_id.is_empty() {
                Ok(User {
                    id: token_data.claims.user_id,
                    role: "user".to_string(),
                    created_at: Utc::now(),
                })
            } else {
                get_or_create_user(&state.sqlite_provider.db, &token_data.claims.sub).await
            }
        } else {
            // Case 2: No token is present. Use the guest user.
            info!("No Authorization header found, using guest user.");
            get_or_create_user(&state.sqlite_provider.db, GUEST_USER_IDENTIFIER).await
        }
        .map_err(|e| {
            // This is an internal error because the DB should be available.
            error!("Failed to get or create user: {}", e);
            AuthError(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Could not retrieve user: {e}"),
            )
        })?;

        // If all checks pass, return the authenticated user (either real or guest).
        Ok(AuthenticatedUser(user))
    }
}
