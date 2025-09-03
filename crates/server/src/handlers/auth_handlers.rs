//! # Authentication Route Handlers
//!
//! This module contains the Axum handlers for the OAuth 2.0 authentication flow,
//! enabling the CLI to perform a secure, browser-based login.

use crate::{errors::AppError, state::AppState};
use anyhow::Context;
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, PrivateCookieJar};
use core_access::get_or_create_user;
use jsonwebtoken::{encode, EncodingKey, Header};
use openidconnect::{
    core::{CoreClient, CoreIdTokenClaims, CoreProviderMetadata, CoreResponseType},
    reqwest::async_http_client,
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};

/// The claims for our application-specific JWT.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// The subject of the token (our internal user ID).
    pub sub: String,
    /// The expiration timestamp.
    pub exp: usize,
}

#[derive(Deserialize)]
pub struct AuthRequest {
    // The port the CLI's local server is listening on.
    pub cli_port: u16,
}

#[derive(Deserialize)]
pub struct AuthCallback {
    code: AuthorizationCode,
    state: CsrfToken,
}

/// A struct to hold the temporary state needed during the OAuth flow.
/// This is serialized and stored in a secure, short-lived cookie.
#[derive(Serialize, Deserialize)]
struct OAuthState {
    pkce_verifier: PkceCodeVerifier,
    csrf_token: CsrfToken,
    nonce: Nonce,
    cli_port: u16,
}

/// Creates a configured OpenID Connect client for Google.
///
/// This helper function centralizes the client creation logic, reading the
/// necessary configuration from environment variables.
async fn create_google_oidc_client() -> Result<CoreClient, AppError> {
    let google_client_id = ClientId::new(
        env::var("GOOGLE_OAUTH_CLIENT_ID")
            .context("Missing GOOGLE_OAUTH_CLIENT_ID environment variable.")?,
    );
    let google_client_secret = ClientSecret::new(
        env::var("GOOGLE_OAUTH_CLIENT_SECRET")
            .context("Missing GOOGLE_OAUTH_CLIENT_SECRET environment variable.")?,
    );
    let server_base_url = env::var("SERVER_BASE_URL")
        .context("Missing SERVER_BASE_URL environment variable (e.g., http://localhost:9090)")?;

    let issuer_url =
        IssuerUrl::new("https://accounts.google.com".to_string()).context("Invalid issuer URL")?;

    let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, async_http_client)
        .await
        .context("Failed to discover Google OIDC provider metadata")?;

    let redirect_url = RedirectUrl::new(format!("{server_base_url}/auth/callback/google"))
        .context("Invalid redirect URL")?;

    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        google_client_id,
        Some(google_client_secret),
    )
    .set_redirect_uri(redirect_url);

    Ok(client)
}

/// Initiates the Google OAuth 2.0 login flow.
///
/// This handler is called when the CLI opens the browser. It constructs the
/// Google authorization URL with the necessary parameters (PKCE, state, scopes)
/// and redirects the user's browser to it. The PKCE verifier and CSRF token are
/// stored in a short-lived, secure cookie to be verified in the callback.
pub async fn google_login_handler(
    jar: CookieJar,
    Query(query): Query<AuthRequest>,
) -> Result<impl IntoResponse, AppError> {
    let client = create_google_oidc_client().await?;
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token, nonce) = client
        .authorize_url(
            AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    let state = OAuthState {
        pkce_verifier,
        csrf_token,
        nonce,
        cli_port: query.cli_port,
    };

    let state_json = serde_json::to_string(&state).context("Failed to serialize OAuth state")?;

    // Store the state in a secure, short-lived cookie.
    let cookie = Cookie::build(("oauth_state", state_json))
        .path("/")
        .http_only(true)
        .secure(false) // `true` for production with HTTPS
        .same_site(cookie::SameSite::Lax)
        .max_age(cookie::time::Duration::minutes(5));

    info!("Redirecting user to Google for authentication.");
    Ok((jar.add(cookie), Redirect::to(auth_url.as_str())))
}

/// Handles the callback from Google after the user has authenticated.
///
/// Google redirects the user here with an authorization code. This handler
/// exchanges that code for an ID token, creates or finds a user in the local

/// database, generates a new application-specific JWT, and redirects the user
/// back to the CLI's local server with the JWT.
pub async fn google_auth_callback_handler(
    State(app_state): State<AppState>,
    mut jar: CookieJar,
    Query(query): Query<AuthCallback>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Retrieve and verify the state from the cookie.
    let state_cookie = jar
        .get("oauth_state")
        .context("Missing 'oauth_state' cookie.")?;
    let state_json = state_cookie.value();
    let oauth_state: OAuthState = serde_json::from_str(state_json)
        .context("Failed to deserialize OAuth state from cookie")?;

    // Remove the cookie immediately to prevent reuse.
    jar = jar.remove(Cookie::named("oauth_state"));

    if oauth_state.csrf_token.secret() != query.state.secret() {
        warn!("CSRF token mismatch.");
        return Err(AppError::Internal(anyhow::anyhow!(
            "CSRF validation failed."
        )));
    }

    // 2. Exchange the authorization code for an ID token.
    let client = create_google_oidc_client().await?;
    let token_response = client
        .exchange_code(query.code)
        .set_pkce_verifier(oauth_state.pkce_verifier)
        .request_async(async_http_client)
        .await
        .context("Failed to exchange authorization code for token")?;

    // 3. Verify the ID token and its claims.
    let id_token = token_response
        .id_token()
        .context("Google did not return an ID token")?;
    let claims: &CoreIdTokenClaims = id_token
        .claims(&client.id_token_verifier(), &oauth_state.nonce)
        .context("Failed to verify ID token claims")?;

    let user_identifier = claims.subject().as_str();
    info!(
        "Successfully validated user from Google: {}",
        user_identifier
    );

    // 4. Get or create a user in our local database.
    let user = get_or_create_user(&app_state.sqlite_provider.db, user_identifier).await?;
    info!("Provisioned local user with ID: {}", user.id);

    // 5. Generate our own application JWT.
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System time is before UNIX epoch")?
        .as_secs()
        + (30 * 24 * 60 * 60); // 30 days
    let jwt_claims = Claims {
        sub: user.id.clone(),
        exp: expiration as usize,
    };
    let jwt_secret = env::var("JWT_SECRET").context("Missing JWT_SECRET environment variable.")?;
    let token = encode(
        &Header::default(),
        &jwt_claims,
        &EncodingKey::from_secret(jwt_secret.as_ref()),
    )
    .context("Failed to sign application JWT")?;

    // 6. Redirect back to the CLI's local server.
    let redirect_url = format!(
        "http://127.0.0.1:{}/oauth/callback?token={}",
        oauth_state.cli_port, token
    );

    info!("Redirecting back to CLI at: {}", redirect_url);
    Ok((jar, Redirect::to(&redirect_url)))
}
