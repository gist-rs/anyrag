use anyrag::SearchResult;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct IngestGitHubRequest {
    pub url: String,
    pub version: Option<String>,
}

#[derive(Serialize)]
pub struct IngestGitHubResponse {
    pub message: String,
    pub ingested_examples: usize,
    pub version: String,
}

#[derive(Deserialize)]
pub struct GetVersionedExamplesPath {
    pub repo_name: String,
    pub version: String,
}

#[derive(Deserialize)]
pub struct GetLatestExamplesPath {
    pub repo_name: String,
}

#[derive(Serialize)]
pub struct GetExamplesResponse {
    pub content: String,
}

#[derive(Deserialize)]
pub struct SearchExamplesRequest {
    pub query: String,
    pub repos: Vec<String>,
}

#[derive(Serialize)]
pub struct SearchExamplesResponse {
    pub results: Vec<SearchResult>,
}
