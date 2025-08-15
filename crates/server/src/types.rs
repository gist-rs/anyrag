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
