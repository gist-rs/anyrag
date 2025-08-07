use crate::{
    errors::PromptError,
    providers::{bigquery::BigQueryProvider, storage::Storage},
};
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug};

/// A client to interact with the Gemini API and a storage provider.
pub struct PromptClient {
    pub(crate) gemini_client: ReqwestClient,
    pub(crate) storage_provider: Box<dyn Storage>,
    pub(crate) gemini_url: String,
    pub(crate) gemini_api_key: String,
}

impl Debug for PromptClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PromptClient")
            .field("gemini_url", &self.gemini_url)
            .field("storage_provider", &self.storage_provider)
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
    storage_provider: Option<Box<dyn Storage>>,
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

    /// Sets the storage provider instance.
    pub fn storage_provider(mut self, storage_provider: Box<dyn Storage>) -> Self {
        self.storage_provider = Some(storage_provider);
        self
    }

    /// A helper to build and set a `BigQueryProvider` as the storage provider.
    pub async fn bigquery_storage(mut self, project_id: String) -> Result<Self, PromptError> {
        let provider = BigQueryProvider::new(project_id).await?;
        self.storage_provider = Some(Box::new(provider));
        Ok(self)
    }

    /// Builds the `PromptClient`.
    ///
    /// This method consumes the builder and returns a `Result` containing
    /// either a configured `PromptClient` or a `PromptError` if configuration
    /// is incomplete (e.g., missing API key or storage provider).
    pub fn build(self) -> Result<PromptClient, PromptError> {
        if self.gemini_api_key.is_empty() {
            return Err(PromptError::MissingApiKey);
        }
        let storage_provider = self
            .storage_provider
            .ok_or(PromptError::MissingStorageProvider)?;

        let gemini_client = ReqwestClient::builder()
            .build()
            .map_err(PromptError::ReqwestClientBuild)?;

        Ok(PromptClient {
            gemini_client,
            storage_provider,
            gemini_url: self.gemini_url,
            gemini_api_key: self.gemini_api_key,
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
