use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

#[derive(Deserialize, Debug)]
pub struct GenTextRequest {
    #[serde(default)]
    pub db: Option<String>,
    pub generation_prompt: String,
    #[serde(default)]
    pub context_prompt: Option<String>,
    #[serde(default)]
    pub model: Option<String>,

    // New Control Flags
    #[serde(default)]
    pub use_sql: bool,
    #[serde(default)]
    pub use_knowledge_search: bool,
    #[serde(default = "default_true")]
    pub use_keyword_search: bool,
    #[serde(default = "default_true")]
    pub use_vector_search: bool,
    pub rerank_limit: Option<u32>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AgentDecision {
    pub tool: String,
    pub query: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DeconstructedQuery {
    pub search_query: String,
    #[serde(default)]
    pub generative_intent: String,
}
