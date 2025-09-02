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
use core_access::{get_or_create_user, User};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::warn;

use crate::state::AppState;

/// Represents the claims we expect to find in the JWT.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// The subject of the token, which we use as the unique user identifier.
    pub sub: String,
    /// The expiration timestamp.
    pub exp: usize,
}

/// An Axum extractor that provides the currently authenticated user.
///
/// When used as an argument in a handler, it triggers the JWT validation logic.
/// If the token is valid, the handler receives the `User` object. If the token
/// is missing or invalid, a `401 Unauthorized` response is sent automatically.
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
        // 1. Extract the token from the `Authorization: Bearer <token>` header.
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
                .await
                .map_err(|e| {
                    warn!("Failed to extract Authorization header: {}", e);
                    AuthError(
                        StatusCode::UNAUTHORIZED,
                        "Missing or invalid Authorization header.".to_string(),
                    )
                })?;

        // 2. Decode and validate the JWT.
        // In a production application, this secret should be loaded securely from config.
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

        // 3. Get or create a user in the database based on the token's subject claim.
        let user = get_or_create_user(&state.sqlite_provider.db, &token_data.claims.sub)
            .await
            .map_err(|e| {
                // This is an internal error because the DB should be available.
                AuthError(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Could not retrieve user: {e}"),
                )
            })?;

        // 4. If all checks pass, return the authenticated user.
        Ok(AuthenticatedUser(user))
    }
}
