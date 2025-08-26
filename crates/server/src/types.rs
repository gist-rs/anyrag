use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Default)]
pub struct DebugParams {
    pub debug: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct ApiResponse<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<Value>,
    pub result: T,
}

// --- Ingestion Payloads ---

#[derive(Deserialize)]
pub struct IngestTextRequest {
    pub text: String,
    #[serde(default = "default_source")]
    pub source: String,
}

fn default_source() -> String {
    "text_input".to_string()
}

#[derive(Serialize)]
pub struct IngestTextResponse {
    pub message: String,
    pub ingested_chunks: usize,
}
