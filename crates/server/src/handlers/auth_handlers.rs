//! # Authentication Route Handlers
//!
//! This module contains the Axum handlers for the OAuth 2.0 authentication flow.
//! NOTE: This is a placeholder implementation to allow the application to compile.

use crate::{auth::middleware::AuthenticatedUser, errors::AppError};
use axum::{
    extract::Query,
    response::{IntoResponse, Json, Redirect},
};
use core_access::User;
use serde::{Deserialize, Serialize};
use tracing::warn;

/// The claims for our application-specific JWT.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// The subject of the token (our internal user ID).
    pub sub: String,
    /// The expiration timestamp.
    pub exp: usize,
}

// Placeholder structs to satisfy the router's handler signatures.
#[derive(Deserialize)]
pub struct AuthRequest {
    #[allow(dead_code)]
    pub cli_port: u16,
}

#[derive(Deserialize)]
pub struct AuthCallback {
    // These fields are not used in the placeholder.
}

/// Initiates the Google OAuth 2.0 login flow (Placeholder).
pub async fn google_login_handler(Query(_query): Query<AuthRequest>) -> impl IntoResponse {
    warn!("google_login_handler is a placeholder and is not functional.");
    // Redirect to Google's main page as a non-functional placeholder.
    Redirect::to("https://google.com")
}

/// Handles the callback from Google after the user has authenticated (Placeholder).
pub async fn google_auth_callback_handler(Query(_query): Query<AuthCallback>) -> impl IntoResponse {
    warn!("google_auth_callback_handler is a placeholder and is not functional.");
    Redirect::to("http://localhost:8080/login_success.html") // A generic success-like page
}

/// Returns the details of the currently authenticated user.
pub async fn get_me_handler(user: AuthenticatedUser) -> Result<Json<User>, AppError> {
    // This handler can remain functional as it just returns the user from the extractor.
    Ok(Json(user.0))
}
