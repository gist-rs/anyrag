use crate::errors::PromptError;
use gcp_bigquery_client::{model::table_schema::TableSchema, Client};
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A client to interact with the Gemini API and BigQuery.
pub struct PromptClient {
    pub(crate) gemini_client: ReqwestClient,
    pub(crate) bigquery_client: Client,
    pub(crate) gemini_url: String,
    pub(crate) gemini_api_key: String,
    pub(crate) project_id: String,
    pub(crate) schema_cache: Arc<RwLock<HashMap<String, Arc<TableSchema>>>>,
}

impl fmt::Debug for PromptClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PromptClient")
            .field("gemini_url", &self.gemini_url)
            .field("project_id", &self.project_id)
            .finish_non_exhaustive()
    }
}

/// A builder for creating `PromptClient` instances.
///
/// This builder facilitates the creation of a `PromptClient` by allowing
/// for the configuration of necessary parameters such as API keys and project IDs.
#[derive(Default)]
pub struct PromptClientBuilder {
    gemini_url: String,
    gemini_api_key: String,
    project_id: String,
}

impl PromptClientBuilder {
    /// Creates a new `PromptClientBuilder`.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyquery::PromptClientBuilder;
    ///
    /// let builder = PromptClientBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the Gemini API URL.
    pub fn gemini_url(mut self, gemini_url: String) -> Self {
        self.gemini_url = gemini_url;
        self
    }

    /// Sets the Gemini API key.
    pub fn gemini_api_key(mut self, gemini_api_key: String) -> Self {
        self.gemini_api_key = gemini_api_key;
        self
    }

    /// Sets the BigQuery project ID.
    pub fn project_id(mut self, project_id: String) -> Self {
        self.project_id = project_id;
        self
    }

    /// Builds the `PromptClient`.
    ///
    /// This method consumes the builder and returns a `Result` containing
    /// either a configured `PromptClient` or a `PromptError` if configuration
    /// is incomplete (e.g., missing API key or project ID).
    pub async fn build(self) -> Result<PromptClient, PromptError> {
        if self.gemini_api_key.is_empty() {
            return Err(PromptError::MissingApiKey);
        }
        if self.project_id.is_empty() {
            return Err(PromptError::MissingProjectId);
        }

        let gemini_client = ReqwestClient::builder()
            .build()
            .map_err(PromptError::ReqwestClientBuild)?;

        let bigquery_client = Client::from_application_default_credentials()
            .await
            .map_err(PromptError::BigQueryClient)?;

        Ok(PromptClient {
            gemini_client,
            bigquery_client,
            gemini_url: self.gemini_url,
            gemini_api_key: self.gemini_api_key,
            project_id: self.project_id,
            schema_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}

#[derive(Serialize)]
pub(crate) struct GeminiRequest {
    pub(crate) contents: Vec<Content>,
}

#[derive(Serialize)]
pub(crate) struct Content {
    pub(crate) parts: Vec<Part>,
}

#[derive(Serialize)]
pub(crate) struct Part {
    pub(crate) text: String,
}

#[derive(Deserialize)]
pub(crate) struct GeminiResponse {
    pub(crate) candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
pub(crate) struct Candidate {
    pub(crate) content: ContentResponse,
}

#[derive(Deserialize)]
pub(crate) struct ContentResponse {
    pub(crate) parts: Vec<PartResponse>,
}

#[derive(Deserialize)]
pub(crate) struct PartResponse {
    pub(crate) text: String,
}
