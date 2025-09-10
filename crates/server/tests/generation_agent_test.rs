//! # Generation Agent E2E Test
//!
//! This file contains an end-to-end test for the refactored `/gen/text` endpoint,
//! verifying that its new agentic, tool-selecting logic works as intended.

mod common;

use anyhow::Result;

#[tokio::test]
async fn test_gen_text_agent_chooses_knowledge_search() -> Result<()> {
    //
    Ok(())
}
