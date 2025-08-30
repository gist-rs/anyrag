# Plan: Integrate Knowledge Graph into Server Search

This document outlines the plan to integrate the existing `KnowledgeGraph` from the `anyrag` library into the `anyrag-server`, providing users with new ways to query the stored knowledge.

The implementation will be broken down into two main phases:

1.  **Phase 1: Knowledge Graph Only Search:** Create a new, dedicated endpoint that exclusively queries the knowledge graph. This will serve as the foundation.
2.  **Phase 2: Hybrid Search Integration:** Enhance the existing `/search/knowledge` endpoint to optionally include the knowledge graph's findings in its RAG context, allowing users to choose whether to use it.

---

## Phase 1: Implement "Knowledge Graph Only" Search

This phase focuses on creating a direct way to interact with the knowledge graph via the API.

### Step 1: Update Application State (`anyrag-server`)

The knowledge graph needs to be a shared resource available to all request handlers.

-   **File:** `anyrag/crates/server/src/state.rs`
-   **Actions:**
    1.  Import `MemoryKnowledgeGraph` from the `anyrag` library.
    2.  Add a `knowledge_graph` field to the `AppState` struct, wrapped in `Arc<RwLock<...>>` for thread-safe, shared, mutable access.
    3.  In the `build_app_state` function, initialize a new `MemoryKnowledgeGraph` and include it in the returned `AppState`.

### Step 2: Create New API Endpoint (`anyrag-server`)

A new handler and route are needed to expose the knowledge graph's query functionality.

-   **File:** `anyrag/crates/server/src/handlers.rs`
    -   **Action 1:** Create a new request payload struct, `KnowledgeGraphSearchRequest`, to accept `subject` and `predicate` strings.
    -   **Action 2:** Create a new response payload struct, `KnowledgeGraphSearchResponse`, to return the found `object` string (or `None`).
    -   **Action 3:** Implement a new handler function, `knowledge_graph_search_handler`, which will:
        -   Accept the `KnowledgeGraphSearchRequest` as a JSON payload.
        -   Acquire a read lock on the `knowledge_graph` from the `AppState`.
        -   Call the graph's `get_fact_as_of` method using the provided subject, predicate, and the current time (`Utc::now()`).
        -   Return the result wrapped in the `KnowledgeGraphSearchResponse`.

-   **File:** `anyrag/crates/server/src/router.rs`
    -   **Action:** Add a new route to the main router: `.route("/search/knowledge_graph", post(knowledge_graph_search_handler))`.

### Step 3: Create an End-to-End Test (`anyrag-server`)

We will validate the new endpoint by creating an end-to-end test that reuses the core logic from the library's existing `knowledge_graph_test.rs`.

-   **File to be created:** `anyrag/crates/server/tests/knowledge_graph_e2e_test.rs`
-   **Actions:**
    1.  **Modify Test Harness:** First, update the `TestApp` struct in `anyrag/crates/server/tests/common.rs` to include the `MemoryKnowledgeGraph` in its state. Expose a public field to allow tests to access and manipulate the graph directly.
    2.  **Create New Test Function:** In the new test file, create a test named `test_knowledge_graph_endpoint_e2e`.
    3.  **Arrange:**
        -   Spawn the `TestApp`.
        -   Directly access the `knowledge_graph` instance from the test harness.
        -   Acquire a write lock and use the `add_fact` method to populate the graph with time-sensitive data (e.g., a "current role" and a "past role").
    4.  **Act:**
        -   Use the `app.client` to send a `POST` request to the new `/search/knowledge_graph` endpoint.
        -   The JSON payload should contain a `subject` and `predicate` that matches the "current role" data.
    5.  **Assert:**
        -   Verify that the HTTP response status is successful (200 OK).
        -   Parse the JSON response and assert that the `object` field contains the expected value for the current role.

---

## Phase 2: Integrate into Hybrid Search

This phase will make the knowledge graph a part of the main RAG pipeline, providing users with more powerful and context-aware answers.

### Step 4: Enhance the Hybrid Search Handler (`anyrag-server`)

-   **File:** `anyrag/crates/server/src/handlers.rs`
-   **Actions:**
    1.  Modify the `SearchRequest` struct to include a new optional boolean field: `use_knowledge_graph`.
    2.  In the `knowledge_search_handler` (the primary RAG endpoint):
        -   Before fetching from the vector/keyword stores, check if `use_knowledge_graph` is `true`.
        -   If it is, perform a query against the `knowledge_graph`. A simple first-pass implementation can use the user's `query` as the `subject` and a default `predicate` like "description" or "role".
        -   If a fact is found, prepend it to the `context` string that is sent to the final LLM for synthesis. The prepended text should be clearly marked, e.g., `"Definitive Answer from Knowledge Graph: [Fact Object].\n\nAdditional Context from Articles:\n..."`.

### Step 5: Update the E2E Test (`anyrag-server`)

-   **File:** `anyrag/crates/server/tests/knowledge_graph_e2e_test.rs`
-   **Actions:**
    1.  Add a new test case, `test_hybrid_search_with_knowledge_graph_context`.
    2.  **Arrange:**
        -   Seed the `knowledge_graph` with a known, unique fact.
        -   Seed the regular database tables with other, less relevant articles.
        -   Use `httpmock` to mock the final RAG synthesis LLM call. The mock's expectation should be configured to check that the request body **must contain** the unique fact from the knowledge graph.
    3.  **Act:** Call the `/search/knowledge` endpoint with a query matching the fact and `use_knowledge_graph: true`.
    4.  **Assert:**
        -   Verify the final API response is successful.
        -   Assert that the `httpmock` for the RAG synthesis call was hit. This is the primary assertion, as it proves the context was correctly augmented by the knowledge graph.