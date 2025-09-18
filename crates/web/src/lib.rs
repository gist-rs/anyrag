//! # anyrag-web: Web Ingestion Plugin
//!
//! This crate provides the ingestion logic for web URLs, acting as a plugin
//! for the `anyrag` ecosystem. It implements the `Ingestor` trait.

use anyrag::{
    ingest::{
        knowledge::{extract_and_store_metadata, restructure_with_llm, YamlContent},
        IngestError, IngestionPrompts, IngestionResult, Ingestor,
    },
    providers::ai::AiProvider,
    PromptError,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn};
use turso::{params, Database};
use uuid::Uuid;

// --- Error Definitions ---

#[derive(Error, Debug)]
pub enum WebIngestError {
    #[error("Database error: {0}")]
    Database(#[from] turso::Error),
    #[error("Failed to fetch content: {0}")]
    Fetch(#[from] reqwest::Error),
    #[error("LLM processing failed: {0}")]
    Llm(#[from] PromptError),
    #[error("Failed to parse LLM response: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("Content has not changed for URL: {0}, skipping.")]
    ContentUnchanged(String),
    #[error("Jina Reader API request failed with status {status}: {body}")]
    JinaReaderFailed { status: u16, body: String },
    #[error("An internal error occurred: {0}")]
    Internal(#[from] anyhow::Error),
    #[error("HTML processing error: {0}")]
    Html(String),
}

impl From<WebIngestError> for IngestError {
    fn from(err: WebIngestError) -> Self {
        match err {
            WebIngestError::Database(e) => IngestError::Database(e),
            WebIngestError::ContentUnchanged(url) => {
                IngestError::Parse(format!("Content unchanged for URL: {url}"))
            }
            WebIngestError::Fetch(e) => IngestError::Fetch(e.to_string()),
            _ => IngestError::Internal(anyhow::anyhow!(err.to_string())),
        }
    }
}

// --- Data Structures ---

/// Defines the strategy for fetching web content.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WebIngestStrategy<'a> {
    #[default]
    RawHtml,
    Jina {
        #[serde(borrow)]
        api_key: Option<&'a str>,
    },
}

#[derive(Deserialize)]
struct IngestSource<'a> {
    url: &'a str,
    #[serde(default)]
    #[serde(borrow)]
    strategy: WebIngestStrategy<'a>,
}

#[derive(Debug, Clone)]
struct IngestedDocument {
    #[allow(unused)]
    id: String,
    content: String,
}

// --- Core Pipeline Logic (Moved from anyrag-lib) ---

pub async fn fetch_web_content(
    url: &str,
    strategy: WebIngestStrategy<'_>,
) -> Result<String, WebIngestError> {
    match strategy {
        WebIngestStrategy::RawHtml => {
            info!("Fetching and cleaning HTML from: {url}");
            anyrag_html::url_to_clean_markdown(url, None)
                .await
                .map_err(|e| WebIngestError::Html(e.to_string()))
        }
        WebIngestStrategy::Jina { api_key } => {
            let fetch_url = format!("https://r.jina.ai/{url}");
            info!("Fetching clean markdown from: {fetch_url}");
            let client = reqwest::Client::new();
            let mut request_builder = client.get(&fetch_url);
            if let Some(key) = api_key {
                if !key.is_empty() {
                    request_builder =
                        request_builder.header("Authorization", format!("Bearer {key}"));
                }
            }
            let response = request_builder.send().await?;
            if !response.status().is_success() {
                let status = response.status().as_u16();
                let body = response.text().await.unwrap_or_default();
                return Err(WebIngestError::JinaReaderFailed { status, body });
            }
            let markdown = response.text().await.map_err(WebIngestError::Fetch)?;
            Ok(anyrag_html::clean_markdown_content(&markdown))
        }
    }
}

async fn ingest_and_cache_url(
    db: &Database,
    url: &str,
    owner_id: Option<&str>,
    strategy: WebIngestStrategy<'_>,
) -> Result<(String, IngestedDocument), WebIngestError> {
    let conn = db.connect()?;
    let markdown_content = fetch_web_content(url, strategy).await?;
    let new_content_hash = format!("{:x}", md5::compute(markdown_content.as_bytes()));

    let title = markdown_content
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim_start_matches(|c: char| c == '#' || c.is_whitespace())
                .trim()
                .chars()
                .take(150)
                .collect::<String>()
        })
        .unwrap_or_else(|| url.to_string());

    if let Some(row) = conn
        .query(
            "SELECT id, content FROM documents WHERE source_url = ?",
            params![url],
        )
        .await?
        .next()
        .await?
    {
        let doc_id: String = row.get(0)?;
        let existing_content: String = row.get(1)?;
        let existing_hash = format!("{:x}", md5::compute(existing_content.as_bytes()));
        if existing_hash == new_content_hash {
            return Err(WebIngestError::ContentUnchanged(url.to_string()));
        }
        conn.execute(
            "UPDATE documents SET content = ?, title = ? WHERE id = ?",
            params![markdown_content.clone(), title, doc_id.clone()],
        )
        .await?;
        let ingested_document = IngestedDocument {
            id: doc_id.clone(),
            content: markdown_content,
        };
        return Ok((doc_id, ingested_document));
    }

    let document_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, url.as_bytes()).to_string();
    conn.execute(
        "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?, ?, ?, ?, ?)",
        params![
            document_id.clone(),
            owner_id,
            url,
            title,
            markdown_content.clone()
        ],
    )
    .await?;
    let ingested_document = IngestedDocument {
        id: document_id.clone(),
        content: markdown_content,
    };
    Ok((document_id, ingested_document))
}

async fn run_web_ingestion_pipeline(
    db: &Database,
    ai_provider: &dyn AiProvider,
    url: &str,
    owner_id: Option<&str>,
    prompts: IngestionPrompts<'_>,
    web_ingest_strategy: WebIngestStrategy<'_>,
) -> Result<usize, WebIngestError> {
    let (temp_doc_id, ingested_document) =
        match ingest_and_cache_url(db, url, owner_id, web_ingest_strategy).await {
            Ok(content) => content,
            Err(WebIngestError::ContentUnchanged(url)) => {
                info!("Content for {} is unchanged, pipeline finished.", url);
                return Ok(0);
            }
            Err(e) => return Err(e),
        };

    let structured_yaml = restructure_with_llm(
        ai_provider,
        &ingested_document.content,
        prompts.restructuring_system_prompt,
    )
    .await
    .map_err(|e| WebIngestError::Internal(anyhow::anyhow!(e)))?;

    if structured_yaml.trim().is_empty() {
        warn!(
            "LLM restructuring resulted in empty content for source: {}",
            url
        );
        let conn = db.connect()?;
        conn.execute("DELETE FROM documents WHERE id = ?", params![temp_doc_id])
            .await?;
        return Ok(0);
    }

    let conn = db.connect()?;
    conn.execute(
        "DELETE FROM documents WHERE id = ?",
        params![temp_doc_id.clone()],
    )
    .await?;
    let yaml_content: YamlContent = match serde_yaml::from_str(&structured_yaml) {
        Ok(content) => content,
        Err(e) => {
            warn!(
                "Failed to parse structured YAML for source: {}. Error: {}",
                url, e
            );
            return Ok(0);
        }
    };

    let mut chunks_created = 0;
    for (i, section) in yaml_content.sections.into_iter().enumerate() {
        let chunk_content = YamlContent {
            sections: vec![section.clone()],
        };
        let yaml_chunk = match serde_yaml::to_string(&chunk_content) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let chunk_id = Uuid::new_v4().to_string();
        let source_url_with_chunk = format!("{url}#section_{i}");
        conn.execute(
            "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?, ?, ?, ?, ?)",
            params![
                chunk_id.clone(),
                owner_id,
                source_url_with_chunk,
                section.title.clone(),
                yaml_chunk.clone()
            ],
        )
        .await?;
        chunks_created += 1;
        extract_and_store_metadata(
            &conn,
            ai_provider,
            &chunk_id,
            owner_id,
            &yaml_chunk,
            prompts.metadata_extraction_system_prompt,
        )
        .await
        .map_err(|e| WebIngestError::Internal(anyhow::anyhow!(e)))?;
    }
    Ok(chunks_created)
}

// --- Ingestor Implementation ---

/// The Ingestor implementation for public web URLs.
pub struct WebIngestor<'a> {
    db: &'a Database,
    ai_provider: &'a dyn AiProvider,
    prompts: IngestionPrompts<'a>,
}

impl<'a> WebIngestor<'a> {
    pub fn new(
        db: &'a Database,
        ai_provider: &'a dyn AiProvider,
        prompts: IngestionPrompts<'a>,
    ) -> Self {
        Self {
            db,
            ai_provider,
            prompts,
        }
    }
}

#[async_trait]
impl<'a> Ingestor for WebIngestor<'a> {
    async fn ingest(
        &self,
        source: &str,
        owner_id: Option<&str>,
    ) -> Result<IngestionResult, IngestError> {
        let ingest_source: IngestSource = serde_json::from_str(source)
            .map_err(|e| IngestError::Parse(format!("Invalid source JSON for web ingest: {e}")))?;

        let documents_added = run_web_ingestion_pipeline(
            self.db,
            self.ai_provider,
            ingest_source.url,
            owner_id,
            self.prompts,
            ingest_source.strategy,
        )
        .await?;

        Ok(IngestionResult {
            source: ingest_source.url.to_string(),
            documents_added,
            document_ids: vec![],
        })
    }
}
