use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct IngestFirebaseRequest {
    pub project_id: String,
    pub collection: String,
    #[serde(default)]
    pub incremental: bool,
    pub timestamp_field: Option<String>,
    pub limit: Option<i32>,
    pub fields: Option<Vec<String>>,
    #[serde(default)]
    pub use_graph: bool,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Serialize)]
pub struct IngestFirebaseResponse {
    pub message: String,
    pub ingested_documents: usize,
    pub documents_processed_for_metadata: usize,
    pub facts_added_to_graph: Option<usize>,
}
