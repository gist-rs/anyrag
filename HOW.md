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

---

## Common Test Failures and Solutions

This section covers common errors encountered during testing and provides step-by-step solutions based on real-world examples.

### Scenario 1: `database is locked` Errors in Server Tests

This is a frequent issue when running multiple tests in parallel (e.g., with `cargo test`).

-   **Symptom**: The test fails with an error message containing `database is locked`.
-   **Cause**: Multiple test instances are trying to access and write to the same hardcoded database file simultaneously. This violates the principle of test isolation. The `TestApp` harness, if not configured correctly, might create a database with a static or predictable name, leading to this collision.
-   **Solution**: Ensure that your `TestApp` harness creates a completely unique and isolated environment for each test run. This typically involves using a temporary directory for all test-generated files, including the main database and any other directories the application might write to (like `github_db_dir`).

**Example Fix in `TestApp::spawn`:**

The key is to create a new temporary directory for *each test* and construct all necessary paths within it.

```rust
// In: crates/server/tests/common/mod.rs

pub async fn spawn(test_case_name: &str) -> Result<Self> {
    let mock_server = MockServer::start();

    // Create a unique temporary directory for this specific test run.
    let temp_dir = tempdir()?;
    
    // Create all necessary files and subdirectories WITHIN the unique temp directory.
    let db_file = NamedTempFile::new_in(temp_dir.path())?;
    let db_path = db_file.path();
    
    let github_db_dir = temp_dir.path().join("github_ingest");
    std::fs::create_dir(&github_db_dir)?;

    // ...

    // Use these unique paths when generating the config for the test server.
    let config_content = format!(
        r#"
        db_url: "{db_path}"
        github_db_dir: "{github_db_path}"
        # ... other configs
        "#,
        db_path = db_path.to_str().unwrap(),
        github_db_path = github_db_dir.to_str().unwrap(),
        // ...
    );

    // ... rest of the setup
}
```

### Scenario 2: `Request did not match any route or mock`

This is the most common error when a test's mock setup becomes desynchronized from the application's actual behavior.

-   **Symptom**: A test making a request to a mock server (like `httpmock` or `wiremock`) fails with a 500-level error. The error body contains a message like `Request did not match any route or mock`.
-   **Cause**: The application code made an HTTP request that the test was not configured to handle. This usually happens for one of two reasons:
    1.  The request payload (body, headers, path) sent by the application does not *exactly* match what the test's mock is expecting.
    2.  The application logic changed, and it's now making an entirely new, unexpected HTTP call that you haven't created a mock for yet.
-   **Solution**: Follow the "Advanced Troubleshooting" workflow outlined in the main guide. The key is to **log the actual request body** from the application code to see what it's sending, and then update your test's mock to match it perfectly.

**Example Walkthrough (`faq_ingestion_test`):**

1.  **The Failure**: The `faq_ingestion_test` failed with a mock error during the `POST /ingest/pdf` call.
2.  **Add Logging**: By adding a `println!` inside the `LocalAiProvider::generate` function, we logged the exact JSON payload being sent.
3.  **Analyze Log**: The log revealed two things:
    *   The application was making an AI call to "restructure" the PDF content, which the test was not mocking at all.
    *   The content being sent was garbled binary data, not clean text.
4.  **The Fix**:
    *   **Add Missing Mocks**: We added new mocks to the test for the "restructure" and "metadata extraction" AI calls that happen during ingestion.
    *   **Correct Assertions**: We updated the test's assertions to reflect the actual pipeline. For example, instead of expecting the raw PDF text in the database, we asserted that the restructured YAML from our new mock was stored correctly. This brought the test in sync with the application's true behavior.

```rust
// In: crates/server/tests/faq_ingestion_test.rs

// ... setup ...

// ADDED: Mock for the first AI call (restructuring)
let restructure_mock = app.mock_server.mock(|when, then| {
    when.method(Method::POST)
        .body_contains("expert document analyst and editor"); // Match on the system prompt
    then.status(200).json_body(/* ... expected YAML response ... */);
});

// ADDED: Mock for the second AI call (metadata)
let metadata_mock = app.mock_server.mock(|when, then| {
    when.method(Method::POST)
        .body_contains("You are a document analyst"); // Match on the system prompt
    then.status(200).json_body(/* ... expected metadata response ... */);
});

// Mocks for the search part of the workflow remain...

// ... rest of the test ...

// ADDED: Assert that the new mocks were called
restructure_mock.assert();
metadata_mock.assert();
```

### Scenario 3: Mock Assertion Failure (`The number of matching requests was higher than expected`)

This error is a more subtle variation of the "request did not match" problem and indicates an issue with mock ambiguity.

-   **Symptom**: The test panics with an assertion failure from the mock library itself, like `assertion 'left == right' failed: The number of matching requests was higher than expected (expected 1 but was 2)`.
-   **Cause**: A single mock definition is being matched by multiple, different HTTP requests within the same test. This happens when the matcher is too broad. For example, a test workflow might make four separate calls to the same `/v1/chat/completions` endpoint, and a matcher like `.path("/v1/chat/completions")` would match all of them, violating a `.hits(1)` assertion.
-   **Solution**: Make your mock matchers more specific. Instead of matching only by path, add criteria that uniquely identify each request. The best way is to match against the request body.

**Example Walkthrough (`sheet_rag_workflow_test`):**

1.  **The Failure**: The test panicked because a mock expecting 1 hit was matched twice.
2.  **Analysis**: We logged the four distinct AI request bodies made during the test: `restructure`, `metadata`, `query_analysis`, and `rag_synthesis`. The mock for `rag_synthesis` was using a very generic matcher (`.body_contains("Answer Directly First")`), which was likely also matching one of the other three requests.
3.  **The Fix**: We made the matcher for each mock highly specific.
    *   For mocks where the entire payload is predictable, use `.json_body()` with the exact expected JSON.
    *   For mocks where part of the payload is dynamic (like the `context` in a RAG call), use a partial matcher that checks for a unique, static part of the request, like the system prompt.

```rust
// In: crates/server/tests/sheet_rag_workflow_test.rs

// This mock is now highly specific and will only match the RAG synthesis call.
let rag_answer_mock = app.mock_server.mock(|when, then| {
    when.method(Method::POST)
        .path(chat_completions_path)
        // Use a partial matcher on the system prompt, which is unique to this call.
        .json_body_partial(json!({
            "messages": [
                {"role": "system", "content": tasks::RAG_SYNTHESIS_SYSTEM_PROMPT},
            ]
        }).to_string()); // .to_string() is crucial!
    then.status(200).json_body(/* ... */);
});
```

### Sub-Problem: `trait bound is not satisfied` on `json_body_partial`

-   **Symptom**: After adding a `.json_body_partial()` matcher, the test fails to compile with `error[E0277]: the trait bound 'String: From<Value>' is not satisfied`.
-   **Cause**: The `httpmock` library's `json_body_partial` function expects a type that can be converted into a `String` (like `&str` or `String`). The `serde_json::json!` macro, however, produces a `serde_json::Value`. The compiler cannot automatically convert from `Value` to `String`.
-   **Solution**: Explicitly call `.to_string()` on the JSON value before passing it to the matcher.

```rust
// Correct usage:
.json_body_partial(
    json!({ "key": "value" }).to_string()
);
```

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

### Scenario 4: `400 Bad Request` on Ingestion Endpoints

-   **Symptom**: The test fails with `HTTP status client error (400 Bad Request)` when calling an ingestion endpoint like `/ingest/pdf`.
-   **Cause**: The server-side handler for the endpoint expects the request payload to be in a specific format, typically `multipart/form-data`, but the test client is sending it in a different format, such as `application/json`. This format mismatch causes the server to reject the request as it cannot parse the body correctly.
-   **Solution**: Review the signature of the server-side handler for the endpoint to determine the expected request format. Update the test client to construct and send the request in that format. For ingestion endpoints that handle file uploads or URL-based ingestion, `multipart` is a common choice.

**Example Walkthrough (`pdf_url_ingest_test`):**

1.  **The Failure**: The `test_pdf_url_ingestion_and_rag_workflow` failed with a `400 Bad Request` error.
2.  **Analysis**: An inspection of the `ingest_pdf_handler` on the server showed that it uses an `axum_extra::extract::Multipart` extractor. This means it is designed to parse a `multipart/form-data` body, looking for specific fields like `url`, `file`, and `extractor`. The test, however, was incorrectly sending a single JSON object.
3.  **The Fix**: The test was refactored to use `reqwest::multipart::Form` to build the request body, correctly structuring the data into parts that the server handler could parse.

```rust
// In: crates/server/tests/pdf_url_ingest_test.rs

// --- Incorrect code sending JSON ---
/*
let ingest_res = app
    .client
    .post(app.url("/ingest/pdf"))
    .bearer_auth(token.clone())
    .json(&json!({ "url": pdf_url, "extractor": "local" })) // INCORRECT
    .send()
    .await?;
*/

// --- Corrected code sending a multipart form ---
let form = reqwest::multipart::Form::new()
    .part("url", reqwest::multipart::Part::text(pdf_url)) // Correctly creates a 'url' part
    .part("extractor", reqwest::multipart::Part::text("local"));

let ingest_res = app
    .client
    .post(app.url("/ingest/pdf"))
    .bearer_auth(token.clone())
    .multipart(form) // Uses .multipart() instead of .json()
    .send()
    .await?;
```
