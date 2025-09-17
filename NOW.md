# NOW

This document outlines the current development task.

## Task: Refactor Test Mocking Strategy

The current integration test mocking strategy in `crates/server/tests/common/mod.rs` is brittle because it relies on matching strings within the AI prompt to select the correct mock. This is unreliable as prompts frequently change.

The goal is to refactor the test harness to use a namespaced approach based on the test case name, ensuring mock isolation and improving reliability.

### Plan

1.  **Refactor `TestApp::spawn` in `crates/server/tests/common/mod.rs`**:
    *   Modify the function signature to accept a `test_case_name: &str`.
    *   Remove the generic, string-matching "catch-all" mock. This will enforce that all API calls in tests must have an explicit mock.
    *   Update the mock server URLs in the generated configuration to include the `test_case_name` as a namespace (e.g., `/test_case_name/v1/chat/completions`).

2.  **Update All Integration Tests**:
    *   Search for all usages of `TestApp::spawn()` across the test suite.
    *   Update each call to `TestApp::spawn()` to pass a unique identifier, preferably the name of the test function.
    *   Adjust the mock definitions within each test to target the new namespaced paths.
    *   For tests that were implicitly relying on the old "catch-all" mock, add the necessary explicit mocks for the API calls they perform.