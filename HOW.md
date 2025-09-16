# How We Debugged the Test Suite

This document outlines the process used to diagnose and fix a series of related, non-obvious test failures in the `anyrag-server` crate. The goal is to provide a guide for future engineers and AI assistants to quickly identify and resolve similar issues.

## 1. The Problem: A Pattern of Failing Tests

Multiple tests, including `generation_agent_test` and `github_search_test`, began failing with similar symptoms after changes were made to the application's AI interaction logic.

### Key Symptoms

Across the failing tests, we observed a consistent pattern:

1.  **Incorrect Final Output**: The application's final output was always a generic string, `Default mock response.`, instead of the expected, test-specific result.
    ```
    assertion `left == right` failed: The final generated text did not match the expected output.
      left: String("Default mock response.")
     right: "Generated post about a heartwarming romance."
    ```

2.  **Mock Assertion Failures**: Mocks defined within the test files (`*_mock.assert()`) would panic, indicating they were never called.
    ```
    panicked at 'assertion `left == right` failed: 0 of 1 expected requests matched the mock specification'
    ```

3.  **JSON Parsing Warnings**: Logs showed warnings where the application tried to parse the AI's response as JSON, but failed because the response was the plain string `"Default mock response."`.
    ```
    WARN ... Failed to parse query analysis JSON ... Raw response: 'Default mock response.'
    ```

## 2. Root Cause Analysis: The Overeager Generic Mock

The investigation led to the test harness defined in `crates/server/tests/common/mod.rs`.

Inside the `TestApp::spawn` function, a **generic, low-priority mock** was configured. Its purpose is to act as a fallback, catching any AI API call that isn't explicitly handled by a more specific mock defined within an individual test.

```rust
// anyrag/crates/server/tests/common/mod.rs

// Add a default mock for any chat completion requests that are NOT for query analysis.
mock_server.mock(|when, then| {
    when.method(Method::POST)
        .path("/v1/chat/completions")
        .matches(|req| {
            let body_str =
                String::from_utf8_lossy(req.body.as_deref().unwrap_or_default());
            // This mock should ignore all specific, handled prompts.
            !body_str.contains("expert query analyst")
                && !body_str.contains("strict, factual AI")
                // ... and other exclusions
        });
    then.status(200)
        .json_body(json!({"...": "Default mock response."}]));
});
```

The core issue was that the `matches` closure used a **negative filter** (a blocklist). When new AI prompts were added to the application (e.g., the "expert code search analyst" prompt for GitHub search), this generic mock's filter was **not updated**.

As a result, when the application made an AI call with the new prompt, the following happened:
1. The specific mock defined in the test file (e.g., `github_search_test.rs`) was waiting for the request.
2. The generic mock in `TestApp` also saw the request. Its filter did **not** contain an exclusion for the new prompt, so it matched the request.
3. The test harness selected the generic mock, which responded with `"Default mock response."`.
4. The test-specific mock was never called, leading to the assertion failure.
5. The application logic received the unexpected plain text response, leading to JSON parsing errors and incorrect final output.

## 3. The Solution: Enhance the Generic Mock's Filter

The definitive solution was not to change the individual tests, but to fix the root cause in the shared test harness. We needed to "teach" the generic mock about the new prompts it should ignore.

This was achieved by adding new conditions to the `matches` closure in `crates/server/tests/common/mod.rs`.

### Example Fix for GitHub Search

The GitHub search introduced a prompt containing `"expert code search analyst"`. The fix was to add this to the blocklist:

```rust
// anyrag/crates/server/tests/common/mod.rs

.matches(|req| {
    let body_str = String::from_utf8_lossy(req.body.as_deref().unwrap_or_default());
    !body_str.contains("expert query analyst")
        && !body_str.contains("expert code search analyst") // <-- ADDED THIS LINE
        && !body_str.contains("strict, factual AI")
        // ...
});
```

By adding this exclusion, the generic mock now correctly ignores the GitHub search analysis prompt, allowing the specific mock defined in `github_search_test.rs` to receive the call as intended. This single change fixed all three symptoms of the failure.

## 4. Key Takeaways and Guiding Principles

When debugging mock-related test failures in this project, follow these principles:

1.  **Identify the "Default Response" Symptom**: If you see `"Default mock response."` in test logs or assertion failures, the problem is almost certainly a mock specificity issue.
2.  **Check the Shared Harness First**: The root cause is likely in the generic mock defined in `crates/server/tests/common/mod.rs`, not in the failing test file itself.
3.  **Update the Exclusion List**: The solution is to add a new exclusion to the `matches` filter of the generic mock. Find a unique, stable string from the new AI prompt that the application is sending and add it to the `!body_str.contains(...)` chain.
4.  **Do Not Modify the Test's Mock**: Trying to make the mock in the test file "more specific" will not work if the generic mock is still matching the request. The fix must be applied to the generic mock's filter.