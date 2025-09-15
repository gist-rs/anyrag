# Plan: Implement Raw HTML Ingestion with `htmd`

This plan outlines the steps to add a new web ingestion method using raw HTML converted to Markdown via the `htmd` crate. This new method will become the default, while the existing Jina service will be retained as an optional strategy.

## 1. Add `htmd` Dependency

-   **Action**: Add the `htmd` crate to the `[dependencies]` section of `anyrag/crates/lib/Cargo.toml`.
-   **Reason**: To leverage its HTML-to-Markdown conversion capabilities.

## 2. Introduce a Web Ingestion Strategy Enum

-   **File**: `anyrag/crates/lib/src/ingest/knowledge.rs`
-   **Action**: Create a new public enum `WebIngestStrategy` with two variants: `RawHtml` and `Jina`.
-   **Reason**: To provide a clear, type-safe way to select the ingestion method and to make `RawHtml` the new default.

## 3. Refactor Content Fetching Logic

-   **File**: `anyrag/crates/lib/src/ingest/knowledge.rs`
-   **Action**:
    1.  Rename the existing `fetch_markdown_from_url` function to `fetch_web_content`.
    2.  Update its signature to accept the new `WebIngestStrategy`.
    3.  Implement a `match` statement on the strategy:
        -   For `RawHtml`: Fetch the raw HTML from the URL, then pass it to `htmd::to_md` for conversion.
        -   For `Jina`: Keep the existing logic that calls the Jina Reader API.
    4.  Preserve the current logic that handles direct `.md` file URLs.
-   **Reason**: To centralize fetching logic and support multiple content processing strategies cleanly.

## 4. Update Pipeline Signatures

-   **File**: `anyrag/crates/lib/src/ingest/knowledge.rs`
-   **Action**:
    1.  Modify the `run_ingestion_pipeline` function to accept an optional `WebIngestStrategy`. If `None` is provided, it should default to `WebIngestStrategy::RawHtml`.
    2.  Update the internal call to `ingest_and_cache_url` to pass this strategy down.
    3.  Modify `ingest_and_cache_url` to accept the strategy and pass it to the new `fetch_web_content` function.
-   **Reason**: To integrate the new strategy selection mechanism into the main ingestion workflow.

## 5. Update Example Usage

-   **File**: `anyrag/crates/lib/examples/knowledge.rs`
-   **Action**:
    1.  Modify the call to `run_ingestion_pipeline` to reflect the new default behavior (raw HTML ingestion).
    2.  Add comments and a configurable variable to show how a user could explicitly choose the `Jina` strategy instead.
-   **Reason**: To ensure the example code is up-to-date and clearly demonstrates how to use both the default and alternative ingestion strategies.