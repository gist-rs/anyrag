# HOWTO: Write Integration Tests for `anyrag`

This guide provides a comprehensive walkthrough for creating integration tests for `anyrag` ingestion plugins and server endpoints. The goal is to verify logic in isolation, ensuring it correctly processes source data, interacts with dependencies, and stores data as expected.

## Guiding Principles

1.  **Test in Isolation**: Tests should not depend on external network services. They should test the specific crate or server endpoint directly.
2.  **Mock External Dependencies**: All external services, especially AI models and web downloads, MUST be mocked. This ensures tests are fast, deterministic, and free from network flakiness.
3.  **Verify Final State**: The primary assertion of an ingestion test is to check the final state. For server tests, this includes both the API response and the state of the database.
4.  **Use Temporary Databases**: Each test should run against a fresh, isolated database (e.g., a temporary file or in-memory instance) to prevent tests from interfering with each other.

---

## Step-by-Step Guide for Plugin Tests

This example shows how to test an ingestion plugin's crate directly.

### Step 1: Set Up the Test Environment

Inside your plugin's crate (e.g., `crates/sheets/`), create a `tests` directory.

```sh
mkdir crates/sheets/tests
touch crates/sheets/tests/sheet_ingest_test.rs
```

### Step 2: Add Dev Dependencies

In your plugin's `Cargo.toml`, add `httpmock` for mocking network calls and `anyrag-test-utils` for helpers.

```toml
# In: crates/sheets/Cargo.toml
[dev-dependencies]
httpmock = "0.7.0"
anyrag-test-utils = { path = "../test-utils" }
```

### Step 3: Write the Test Case

Write the test following the "Arrange, Act, Assert" pattern. This pattern is crucial for creating clear, readable, and maintainable tests.

```rust
// In: crates/sheets/tests/sheet_ingest_test.rs
use anyhow::Result;
use anyrag::ingest::{IngestionPrompts, Ingestor};
use anyrag_sheets::SheetsIngestor; // Your plugin's ingestor
use anyrag_test_utils::{MockAiProvider, TestSetup}; // Helpers
use httpmock::{Method, MockServer};
use serde_json::json;
use turso::params;

#[tokio::test]
async fn test_sheet_ingestion_workflow() -> Result<()> {
    // --- 1. Arrange ---
    let setup = TestSetup::new().await?;
    let ai_provider = MockAiProvider::new();
    let mock_server = MockServer::start(); // For mocking downloads
    let owner_id = "test-user-001";

    // Define mock data and expected outcomes
    let csv_content = "question,answer\nWhat is the new feature?,It is the flux capacitor.";
    let expected_yaml = r#"sections:\n  - title: "Sheet Data""#; // (abbreviated)
    let mock_metadata = json!([{"type": "KEYPHRASE", "value": "flux capacitor"}]).to_string();

    // --- 2. Mock External Services ---
    // A. Mock the web server for the CSV download
    let sheet_serve_mock = mock_server.mock(|when, then| {
        when.method(Method::GET).path("/spreadsheets/d/mock_sheet_id/export");
        then.status(200).body(csv_content);
    });

    // B. Program the Mock AI Provider with expected responses for each AI call
    ai_provider.add_response("expert document analyst", expected_yaml);
    ai_provider.add_response("extract Category, Keyphrases", &mock_metadata);

    // --- 3. Act ---
    let prompts = IngestionPrompts { /* ... */ };
    let ingestor = SheetsIngestor::new(&setup.db, &ai_provider, prompts);
    let source = json!({ "url": mock_server.url("/spreadsheets/d/mock_sheet_id/edit") }).to_string();

    let result = ingestor.ingest(&source, Some(owner_id)).await?;

    // --- 4. Assert ---
    // A. Assert the result from the ingestor
    assert_eq!(result.documents_added, 1);
    let doc_id = &result.document_ids[0];

    // B. Assert the final state of the database
    let conn = setup.db.connect()?;
    let stored_content: String = conn.query_row(/* ... */).await?;
    assert_eq!(stored_content.trim(), expected_yaml.trim());

    // C. Assert that mocks were called as expected
    sheet_serve_mock.assert();
    assert_eq!(ai_provider.get_calls().len(), 2, "Expected 2 AI calls");

    Ok(())
}
```

---

## Troubleshooting Guide

### Basic Troubleshooting

-   **Compilation Error: `unresolved import`**: You are using a crate in your test that isn't declared as a dependency. Add the missing crate to the `[dev-dependencies]` section of your plugin's `Cargo.toml`.
-   **Compilation Error: `private item`**: Your test needs to access an item from another crate that is not `pub`. Go to the source crate and make the item public (e.g., change `mod sql;` to `pub mod sql;`).
-   **Test Failure: `MockAiProvider: No response programmed`**: The `ingestor` called the AI provider, but you didn't program a response for the specific `system_prompt` it used. Ensure the key in `ai_provider.add_response("key", ...)` is a unique substring of the system prompt being sent.

### Advanced Troubleshooting: Common Test Failures

This section covers complex issues that often arise in integration tests.

#### Problem: `database is locked` in Parallel Tests

-   **Symptom**: Tests pass when run individually but fail when run as a suite (`cargo test`) with errors like `database is locked` or `SQL execution failure`.
-   **Cause**: Multiple tests are trying to write to the same hardcoded database file simultaneously. The `TestApp` or test setup is creating a database with a predictable, non-unique name.
-   **Solution**: Ensure every test instance gets a completely isolated database. Modify your test harness (e.g., `TestApp::spawn`) to use `tempfile::NamedTempFile` or `tempfile::tempdir` to generate a unique database path for *each* test run. This guarantees that tests cannot interfere with each other's state.

    ```rust
    // In your test harness (e.g., crates/server/tests/common/mod.rs)
    use tempfile::NamedTempFile;

    pub async fn spawn(test_case_name: &str) -> Result<Self> {
        // This creates a new, unique temporary file for each call.
        let db_file = NamedTempFile::new()?;
        let db_path = db_file.path().to_path_buf();

        // Pass this unique `db_path` to your AppState and config.
        // ...
    }
    ```

#### Problem: Mock Failures (`Request did not match any route or mock`)

-   **Symptom**: The test fails with a message like `AI provider returned an error: {"message":"Request did not match any route or mock"}`.
-   **Cause**: The HTTP request payload sent by your application code does not **exactly** match the payload you defined in your test's mock, even if they look similar. This can also happen if the application makes an unexpected AI call that you haven't mocked at all (e.g., an ingestion test that also requires mocks for a search workflow).
-   **Solution**: Follow this workflow to find the exact payload and create a perfect mock.

##### Step 1: Add Temporary Logging to the Source Code

Modify the application code that makes the HTTP call to print the *exact* request body before it's sent. For example, in the `LocalAiProvider`:

```rust
// In: crates/lib/src/providers/ai/local.rs
// Temporarily add `serde_json` to imports if needed.

// ... inside the `generate` function ...
let request_body = LocalAiRequest { /* ... */ };

// ADD THIS LINE TEMPORARILY
println!("-- AI Request Body: {}", serde_json::to_string_pretty(&request_body).unwrap());

let response = request_builder.json(&request_body).send().await;
// ...
```

##### Step 2: Run the Failing Test and Capture the Output

Run the test again. It will still fail, but now the console output will contain the exact, pretty-printed JSON payload that was sent to the mock server.

```sh
cargo test -p anyrag-server --test ingest_sheet_test
```

Look for the `-- AI Request Body:` output in the test logs.

##### Step 3: Compare the Actual Payload with Your Test's Mock

Carefully compare the logged payload with the one you constructed in your test. You will likely find subtle but critical differences.

**Example Scenario**:

-   **Your Test Payload (`restructure_payload`)**:
    ```json
    {
      "model": "mock-local-model",
      "messages": [
        { "role": "system", "content": "..." },
        { "role": "user", "content": "question,answer..." }
      ]
    }
    ```
-   **Actual Logged Payload (from `println!`)**:
    ```json
    {
      "messages": [
        { "role": "system", "content": "..." },
        { "role": "user", "content": "# Markdown Content to Process:\nquestion,answer..." }
      ],
      "model": "mock-gemini-model",
      "temperature": 0.0,
      "max_tokens": 8192,
      "stream": false
    }
    ```

**Discrepancies Found**:
1.  **`model`**: The app used `"mock-gemini-model"`, not `"mock-local-model"`.
2.  **`user.content`**: The app added a prefix (`# Markdown Content to Process:\n`).
3.  **Missing Fields**: The actual payload includes `temperature`, `max_tokens`, and `stream`, which were missing from the test's mock.

##### Step 4: Correct the Mock in Your Test

Update the JSON payload in your test to be an exact match of the logged output. Using a precise matcher like `.json_body()` is now possible and highly recommended for robustness.

```rust
// In: crates/server/tests/ingest_sheet_test.rs
let restructure_payload = json!({
    "model": "mock-gemini-model", // Corrected
    "messages": [
        {"role": "system", "content": "..."},
        // Corrected user content
        {"role": "user", "content": "# Markdown Content to Process:\nquestion,answer..."}
    ],
    // Added missing fields
    "temperature": 0.0,
    "max_tokens": 8192,
    "stream": false
});

let restructure_mock = app.mock_server.mock(|when, then| {
    when.method(Method::POST)
        .path(...)
        .json_body(restructure_payload); // Use a precise matcher
    then.status(200).json_body(...);
});
```

##### Step 5: Remove the Temporary Logging

Once the test passes, **remove the `println!` statement** from the application code.

### Compilation Error: Borrow of Moved Value (`E0382`)

-   **Symptom**: `error[E0382]: borrow of moved value: ingest_result.document_ids`.
-   **Cause**: You moved ownership of a value into one variable, then tried to use (borrow) it from the original variable. This often happens when constructing response structs.
-   **Example**:
    ```rust
    // This code FAILS
    let response = IngestSheetResponse {
        document_ids: ingest_result.document_ids, // `document_ids` is MOVED here
    };

    let debug_info = json!({
        // ERROR: Trying to BORROW `document_ids` after it was moved
        "document_id": ingest_result.document_ids.first(),
    });
    ```
-   **Solution**: Reorder your statements. Create the variables that borrow the value *before* you create the variable that takes ownership (moves the value).
    ```rust
    // This code PASSES
    let debug_info = json!({
        // BORROW happens here first, which is fine
        "document_id": ingest_result.document_ids.first(),
    });

    let response = IngestSheetResponse {
        document_ids: ingest_result.document_ids, // MOVE happens here, which is now safe
    };
    ```
