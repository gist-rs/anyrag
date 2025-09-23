//! # CLI Authentication Module
//!
//! This module handles the OAuth 2.0 Authorization Code Grant flow for the CLI.
//! It orchestrates a seamless, browser-based login experience without requiring
//! the user to manually handle tokens or codes.

use anyhow::{bail, Result};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde::Deserialize;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Mutex};
use tracing::{error, info};

/// The query parameters expected on the callback from the `anyrag-server`.
#[derive(Deserialize, Debug)]
struct CallbackParams {
    token: String,
    error: Option<String>,
}

/// The core authentication service that handles the local server callback.
async fn auth_service(
    req: Request<Incoming>,
    token_tx: Arc<Mutex<Option<oneshot::Sender<Result<String>>>>>,
) -> Result<Response<String>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/oauth/callback") => {
            let params =
                serde_urlencoded::from_str::<CallbackParams>(req.uri().query().unwrap_or(""))
                    .map_err(|e| {
                        error!("Failed to parse callback query parameters: {}", e);
                        // This error case is internal, so we don't create a full hyper::Error
                        Ok::<_, hyper::Error>(Response::new("Bad Request".to_string()))
                    })
                    .unwrap();

            let mut tx_lock = token_tx.lock().await;

            if let Some(tx) = tx_lock.take() {
                if let Some(err_msg) = params.error {
                    let _ = tx.send(Err(anyhow::anyhow!(
                        "Authentication server returned an error: {err_msg}"
                    )));
                    let mut response = Response::new("Login failed. Please try again.".to_string());
                    *response.status_mut() = StatusCode::UNAUTHORIZED;
                    return Ok(response);
                }
                let _ = tx.send(Ok(params.token));
                Ok(Response::new(
                    "Login successful! You can close this browser tab now.".to_string(),
                ))
            } else {
                // This can happen if the callback URL is hit more than once.
                Ok(Response::new(
                    "This login link has already been used.".to_string(),
                ))
            }
        }
        _ => {
            let mut not_found = Response::new("Not Found".to_string());
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

/// Initiates and manages the entire browser-based login flow.
///
/// This function will:
/// 1. Start a local web server on a free port.
/// 2. Open the user's browser to the `anyrag-server`'s login URL.
/// 3. Wait for the server to redirect the user back to the local server.
/// 4. Capture the JWT from the redirect and shut down the local server.
/// 5. Return the received JWT.
pub async fn login() -> Result<String> {
    // 1. Find a free port for the local callback server.
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let local_addr = listener.local_addr()?;
    let port = local_addr.port();
    info!("Local callback server will listen on port {}", port);

    // 2. Construct the login URL and open it in the browser.
    // TODO: The base URL should be configurable.
    let server_base_url = "http://localhost:9090";
    let login_url = format!("{server_base_url}/auth/login/google?cli_port={port}");
    open::that(&login_url)
        .map_err(|e| anyhow::anyhow!("Failed to open browser for authentication: {e}"))?;
    info!("Opened browser to: {}", login_url);

    // 3. Set up the one-shot channel to receive the token from the server.
    let (token_tx, token_rx) = oneshot::channel();
    let token_tx = Arc::new(Mutex::new(Some(token_tx)));

    // 4. Start the local server.
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
    let server_handle = tokio::spawn(async move {
        let mut conn_count = 0;
        loop {
            tokio::select! {
                res = listener.accept() => {
                     if let Ok((stream, _)) = res {
                        conn_count += 1;
                        let io = TokioIo::new(stream);
                        let token_tx_clone = Arc::clone(&token_tx);

                        tokio::task::spawn(async move {
                            if let Err(err) = http1::Builder::new()
                                .serve_connection(io, service_fn(move |req| auth_service(req, Arc::clone(&token_tx_clone))))
                                .await
                            {
                                error!("Error serving connection: {:?}", err);
                            }
                        });

                        // For the CLI flow, we only expect one connection.
                        if conn_count > 0 {
                             let _ = shutdown_tx.send(());
                             break;
                        }
                    }
                }
                _ = &mut shutdown_rx => {
                    info!("Graceful shutdown signal received.");
                    break;
                }
            }
        }
    });

    // 5. Wait for the token or a timeout.
    let token_result = tokio::select! {
        res = token_rx => res.map_err(|e| anyhow::anyhow!("Token channel closed unexpectedly: {e}")),
        _ = tokio::time::sleep(std::time::Duration::from_secs(120)) => {
            server_handle.abort();
            bail!("Authentication flow timed out after 2 minutes.")
        }
    }?;

    let token = token_result?;
    info!("Successfully received token from callback server.");
    server_handle.abort();

    Ok(token)
}
