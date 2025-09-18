# HOWTO: Write Integration Tests for `anyrag` Plugins

This guide provides a comprehensive walkthrough for creating integration tests for `anyrag` ingestion plugins. The goal is to verify a plugin's logic in isolation, ensuring it correctly processes source data, interacts with dependencies, and stores data in the database as expected.

We will use the `anyrag-sheets` plugin as a reference example.

## Guiding Principles

1.  **Test in Isolation**: Plugin tests should not depend on a running server or external network services. They should test the plugin's crate directly.
2.  **Mock External Dependencies**: All external services, especially AI models and web downloads, MUST be mocked. This ensures tests are fast, deterministic, and free from network flakiness.
3.  **Verify Database State**: The primary assertion of an ingestion test is to check the state of the database *after* the ingestion process is complete. Did it store the correct content? Was metadata extracted properly?
4.  **Use In-Memory Databases**: Each test should run against a fresh, isolated, in-memory SQLite database to prevent tests from interfering with each other.

---

## Step-by-Step Guide

### Step 1: Set Up the Test Environment

Inside your plugin's crate (e.g., `crates/my-plugin/`), create the standard Rust test directory structure.

1.  **Create the `tests` directory**:
    ```sh
    mkdir crates/my-plugin/tests
    ```
2.  **Create the main test file**:
    ```sh
    touch crates/my-plugin/tests/ingest_test.rs
    ```
3.  **Create a `common` module for shared utilities**:
    ```sh
    mkdir crates/my-plugin/tests/common
    touch crates/my-plugin/tests/common/mod.rs
    ```

### Step 2: Create Test Utilities in `common/mod.rs`

This file will contain helpers to set up the database and mock the AI provider.

#### A. The `TestSetup` Struct

This helper creates an isolated in-memory database and ensures the application schema is initialized before each test.

```rust
// In: crates/my-plugin/tests/common/mod.rs

use anyhow::Result;
use turso::Database;

/// A helper struct to manage database creation for each test.
pub struct TestSetup {
    pub db: Database,
}

impl TestSetup {
    pub async fn new() -> Result<Self> {
        // Use a unique in-memory DB for each test to ensure isolation.
        let db = turso::Builder::new_local(":memory:").build().await?;

        // Connect and initialize the schema using the SQL constants from `anyrag-lib`.
        let conn = db.connect()?;
        for statement in anyrag::providers::db::sqlite::sql::ALL_TABLE_CREATION_SQL {
            conn.execute(statement, ()).await?;
        }

        Ok(Self { db })
    }
}
```

#### B. The `MockAiProvider`

This is a mock implementation of the `anyrag::providers::ai::AiProvider` trait. It lets you program responses for expected AI calls and verify that the correct calls were made.

```rust
// In: crates/my-plugin/tests/common/mod.rs (continued)

use anyrag::errors::PromptError;
use anyrag::providers::ai::AiProvider;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct MockAiProvider {
    responses: Arc<Mutex<HashMap<String, String>>>,
    calls: Arc<Mutex<Vec<(String, String)>>>,
}

impl MockAiProvider {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(HashMap::new())),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Pre-programs a response for a specific prompt.
    /// The key should be a unique substring of the system prompt.
    pub fn add_response(&self, key: &str, response: &str) {
        let mut responses = self.responses.lock().unwrap();
        responses.insert(key.to_string(), response.to_string());
    }

    /// Retrieves the recorded calls for assertion.
    pub fn get_calls(&self) -> Vec<(String, String)> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl AiProvider for MockAiProvider {
    async fn generate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, PromptError> {
        let mut calls = self.calls.lock().unwrap();
        calls.push((system_prompt.to_string(), user_prompt.to_string()));

        let responses = self.responses.lock().unwrap();
        for (key, response) in responses.iter() {
            if system_prompt.contains(key) {
                return Ok(response.clone());
            }
        }

        Err(PromptError::AiApi(format!(
            "MockAiProvider: No response programmed for system prompt. Got: '{}'",
            system_prompt
        )))
    }
}
```

### Step 3: Write the Test Case

Now, in your `ingest_test.rs`, write the test following the "Arrange, Act, Assert" pattern.

```rust
// In: crates/my-plugin/tests/ingest_test.rs

// Import common modules and necessary items
mod common;

use anyhow::Result;
use anyrag::ingest::Ingestor;
use crate::common::{MockAiProvider, TestSetup}; // Replace with your plugin's Ingestor
use httpmock::{Method, MockServer};
use serde_json::json;
use turso::{params, Value as TursoValue};
use your_plugin::MyIngestor; // Example

#[tokio::test]
async fn test_ingestion_workflow() -> Result<()> {
    // --- 1. Arrange ---
    let setup = TestSetup::new().await?;
    let ai_provider = MockAiProvider::new();
    let mock_server = MockServer::start(); // For mocking downloads
    let owner_id = "test-user-001";

    // Define mock data and expected outcomes
    let mock_source_content = "some data to be ingested";
    let expected_restructured_content = "structured version of the data";
    let mock_metadata_response = `json!([{"type": "KEYPHRASE", "value": "test"}]).to_string()`;

    // --- 2. Mock External Services ---
    // A. Mock a web server if your plugin downloads content
    let download_mock = mock_server.mock(|when, then| {
        when.method(Method::GET).path("/source-data");
        then.status(200).body(mock_source_content);
    });

    // B. Program the Mock AI Provider with expected responses
    ai_provider.add_response("prompt for restructuring", expected_restructured_content);
    ai_provider.add_response("prompt for metadata", &mock_metadata_response);

    // --- 3. Act ---
    let prompts = ... // Define the prompts your ingestor needs
    let ingestor = MyIngestor::new(&setup.db, &ai_provider, prompts);
    let source = json!({ "url": mock_server.url("/source-data") }).to_string();

    let result = ingestor.ingest(&source, Some(owner_id)).await?;

    // --- 4. Assert ---
    // A. Check the result struct
    assert_eq!(result.documents_added, 1);
    let doc_id = &result.document_ids[0];

    // B. Check the database state
    let conn = setup.db.connect()?;
    let stored_content: String = conn.query_row(
        "SELECT content FROM documents WHERE id = ?",
        params![doc_id.clone()],
        |row| row.get(0)
    ).await?;
    assert_eq!(stored_content.trim(), expected_restructured_content.trim());
    
    // C. Check metadata
    let stored_meta_value: String = conn.query_row(
        "SELECT metadata_value FROM content_metadata WHERE document_id = ?",
        params![doc_id.clone()],
        |row| row.get(0)
    ).await?;
    assert_eq!(stored_meta_value, "test");

    // D. Assert mocks were called
    download_mock.assert();
    assert_eq!(ai_provider.get_calls().len(), 2);

    Ok(())
}
```

### Step 4: Configure `Cargo.toml`

Add the necessary testing libraries to your plugin's `Cargo.toml`.

```toml
# In: crates/my-plugin/Cargo.toml

[dev-dependencies]
httpmock = "0.7.0"
# dyn-clone is sometimes needed by mock objects if the trait uses it.
# dyn-clone = "1.0.17" 
```

---

## Troubleshooting Guide

If your tests fail, consult this guide for common solutions.

### Compilation Error: `unresolved import` / `unlinked crate`

-   **Symptom**: `error[E0432]: unresolved import` or `use of unresolved module or unlinked crate`.
-   **Solution**: You are using a crate in your test code that hasn't been declared as a dependency. Add the missing crate (e.g., `httpmock`, `dyn-clone`) to the `[dev-dependencies]` section of your plugin's `Cargo.toml`.

### Compilation Error: `private module` or `private item`

-   **Symptom**: `error[E0603]: module 'sql' is private`.
-   **Solution**: Your test needs to access an item (module, function, constant) from another crate (`anyrag` or a sibling plugin) that is not public. To fix this, you must go to the source crate and make the item public. For example, change `mod sql;` to `pub mod sql;`.

### Test Failure: `MockAiProvider: No response programmed`

-   **Symptom**: The test panics at runtime with an error from the `MockAiProvider`.
-   **Cause**: The `ingestor` called the AI provider's `generate` method, but you did not program a response for the specific `system_prompt` it used.
-   **Solution**:
    1.  **Check Your Key**: The string key you use in `ai_provider.add_response("key", ...)` **must be a unique substring** of the system prompt being sent. Double-check for typos.
    2.  **Verify the Prompt Constant**: Ensure you are passing the correct prompt constant (e.g., `KNOWLEDGE_RESTRUCTURING_SYSTEM_PROMPT`) to your ingestor. An incorrect prompt will lead to a key mismatch.
    3.  **Count Your Calls**: Make sure the number of AI calls your code makes matches the number of responses you've programmed. If you expect two calls (e.g., restructure and metadata), you need two `add_response` calls with the correct keys.