# Plan: Temporal Reasoning for Queries

## 1. Objective

To enhance the application's RAG pipeline to understand and correctly answer queries that involve temporal concepts like "newest," "latest," or "most recent." The system should be able to identify the most current fact from a set of candidates based on a timestamped property.

## 2. The Test Case

We will be guided by the following end-to-end test scenario:

*   **Facts in the Knowledge Base:**
    *   iPhone 16 was released on `2024-09-09`.
    *   iPhone 17 was released on `2025-09-09`.
    *   Alice has an iPhone 16.
*   **User Query:** "Bob wants the newest iPhone, what should he get?"
*   **System's "Today" Date:** `2025-09-12`
*   **Expected Outcome:** The RAG pipeline should use **only** the information about the iPhone 17 to generate its final answer. The expected answer would be something like: "Based on the release dates, Bob should get the iPhone 17."

## 3. Implementation Strategy

### Step 1: Data Modeling for Temporal Properties

We will represent time-sensitive properties using the existing `content_metadata` table. This avoids a complex schema migration.

*   A new `metadata_type` will be introduced: `"PROPERTY"`.
*   The `metadata_subtype` will represent the property's name (e.g., `"release_date"`).
*   The `metadata_value` will store the date or timestamp (e.g., `"2025-09-09"`).

### Step 2: Make Temporal Reasoning Configurable

To avoid hardcoding, we will add a new section to the `config.yml` file.

```yaml
# in config.yml
temporal_reasoning:
  keywords: ["newest", "latest", "most recent"]
  property_name: "release_date"
```

This will require updating the `Config` structs in `anyrag_server/src/config.rs` to load this new configuration into the `AppState`.

### Step 3: Enhance the Hybrid Search Pipeline

The core logic will be implemented in the `hybrid_search` function located in `anyrag/crates/lib/src/search.rs`.

1.  **Detection**: After the initial query analysis, the function will check if any of the `analyzed_query.keyphrases` match the `temporal_reasoning.keywords` from the configuration.

2.  **Temporal Ranking (If Triggered)**:
    *   If a temporal keyword is detected, a new ranking step will be executed *after* the initial candidate retrieval (metadata, keyword, vector searches) and *before* the final RAG synthesis.
    *   This step will fetch the temporal property (e.g., `release_date`) from the `content_metadata` table for each candidate document.
    *   It will then parse these date properties and sort the candidate documents in descending order.
    *   Crucially, it will truncate the result list to the **top 1** candidate to provide the most precise and unambiguous context to the final RAG prompt.

### Step 4: Write and Execute the Test

1.  Create a new test file: `anyrag/crates/server/tests/e2e_temporal_reasoning_test.rs`.
2.  Use the `TestDataBuilder` to seed the database with the iPhone 16 and iPhone 17 data, including their `"release_date"` as a `"PROPERTY"` in the metadata.
3.  Set up the mock AI services. The `rag_synthesis_mock` is critical: it must assert that the context it receives *only* contains information about the iPhone 17.
4.  Run the test, which will fail initially.
5.  Implement the logic from Steps 1-3.
6.  Run the test again and confirm it passes.