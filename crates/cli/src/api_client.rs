//! # API Client
//!
//! This module provides a client for interacting with the `anyrag-server` API.
//! It handles request construction, authentication, and response parsing.

use anyhow::{bail, Result};
use keyring::Entry;
use reqwest::Client;
use serde::Deserialize;
use tracing::info;

const KEYRING_SERVICE: &str = "anyrag-cli";
const KEYRING_USERNAME: &str = "user";

/// A response item from the `GET /documents` endpoint.
#[derive(Clone, Debug, Deserialize)]
#[allow(dead_code)]
pub struct DocumentResponse {
    pub id: String,
    pub owner_id: String,
    pub source_url: String,
    pub title: String,
    pub created_at: String,
}

/// A response item from the `GET /users` endpoint.
#[derive(Clone, Debug, Deserialize)]
pub struct UserListResponse {
    pub id: String,
    pub role: String,
    pub created_at: String,
}

/// A response item from the `GET /auth/me` endpoint.
#[derive(Clone, Debug, Deserialize)]
#[allow(dead_code)]
pub struct UserResponse {
    pub id: String,
    pub role: String,
    pub created_at: String,
}

/// The client for making API calls to the `anyrag-server`.
pub struct ApiClient {
    client: Client,
    base_url: String,
    keyring_entry: Entry,
}

impl ApiClient {
    /// Creates a new `ApiClient`.
    pub fn new(base_url: String) -> Result<Self> {
        let entry = Entry::new(KEYRING_SERVICE, KEYRING_USERNAME)?;
        Ok(Self {
            client: Client::new(),
            base_url,
            keyring_entry: entry,
        })
    }

    /// Fetches the list of documents from the server.
    ///
    /// It automatically retrieves the JWT from the keychain and adds it to the
    /// request as an `Authorization` header. If no token is found, it makes an
    /// unauthenticated request for guest users.
    pub async fn get_documents(&self) -> Result<Vec<DocumentResponse>> {
        let token = self.keyring_entry.get_password().ok();
        let url = format!("{}/documents", self.base_url);
        info!("Fetching documents from: {}", url);

        let mut request_builder = self.client.get(&url);
        if let Some(token) = token {
            request_builder = request_builder.bearer_auth(token);
        }

        let response = request_builder.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            bail!(
                "Failed to fetch documents. Server responded with {}: {}",
                status,
                error_text
            );
        }

        // The /documents endpoint returns a standard ApiResponse
        let api_response: serde_json::Value = response.json().await?;
        let documents: Vec<DocumentResponse> =
            serde_json::from_value(api_response["result"].clone())?;

        Ok(documents)
    }

    /// Fetches the details of the currently authenticated user.
    /// This method requires a token and will fail if the user is not logged in.
    pub async fn get_me(&self) -> Result<UserResponse> {
        let token = self.keyring_entry.get_password().map_err(|e| {
            anyhow::anyhow!("You are not logged in. Cannot fetch user details. Error: {e}")
        })?;

        let url = format!("{}/auth/me", self.base_url);
        info!("Fetching user details from: {}", url);

        let response = self.client.get(&url).bearer_auth(token).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            bail!(
                "Failed to fetch user details. Server responded with {}: {}",
                status,
                error_text
            );
        }

        // The /auth/me endpoint returns the User object directly.
        let user: UserResponse = response.json().await?;
        Ok(user)
    }

    /// Fetches the list of all users from the server (root only).
    pub async fn get_users(&self) -> Result<Vec<UserListResponse>> {
        let token = self.keyring_entry.get_password().map_err(|e| {
            anyhow::anyhow!("You are not logged in. Cannot fetch users. Error: {e}")
        })?;

        let url = format!("{}/users", self.base_url);
        info!("Fetching all users from: {}", url);

        let response = self.client.get(&url).bearer_auth(token).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            bail!(
                "Failed to fetch users. Server responded with {}: {}",
                status,
                error_text
            );
        }

        let api_response: serde_json::Value = response.json().await?;
        let users: Vec<UserListResponse> = serde_json::from_value(api_response["result"].clone())?;

        Ok(users)
    }

    /// Sends a URL to the server for ingestion into the knowledge base.
    pub async fn ingest_url(&self, url_to_ingest: &str) -> Result<()> {
        let token = self.keyring_entry.get_password().ok();
        let url = format!("{}/knowledge/ingest", self.base_url);
        info!("Sending ingest request for: {}", url_to_ingest);

        let mut request_builder = self.client.post(&url);
        if let Some(token) = token {
            request_builder = request_builder.bearer_auth(token);
        }

        let payload = serde_json::json!({ "url": url_to_ingest });
        let response = request_builder.json(&payload).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            bail!(
                "Failed to ingest URL. Server responded with {}: {}",
                status,
                error_text
            );
        }

        Ok(())
    }
}
