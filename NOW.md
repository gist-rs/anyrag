# API Refactoring Plan

This document outlines the plan for refactoring the data ingestion API to make it more consistent, controllable, and intuitive.

## Guiding Principles

The refactoring is guided by two main principles:

1.  **`?faq=true`**: This is an **opt-in** query parameter that triggers the expensive, AI-based generation of new Question/Answer pairs (FAQs) from a source document. By default, this is disabled.
2.  **`?embed=true`**: This is the **default** behavior for all ingestion. It ensures that any ingested content is immediately vectorized and ready for semantic search. This can be disabled with `?embed=false`.

---

## Core Concepts

### 1. Light Ingestion (Default Behavior)

This is the standard, low-cost operation for all `/ingest` endpoints when the `?faq=true` flag is not present.

-   **Steps:**
    1.  Fetch and clean the source data (e.g., extract text from a PDF, get markdown from a URL).
    2.  Store the processed content as a single entry in the `documents` table.
    3.  Generate vector embeddings for the stored content, making it searchable.

### 2. FAQ Generation (`?faq=true`)

This is the explicit, opt-in mode for creating a rich knowledge base.

-   **Steps:**
    1.  Perform all "Light Ingestion" steps.
    2.  Send the processed content to an AI to perform "distillation and augmentation," which generates new, structured Q&A pairs.
    3.  Store these generated Q&A pairs in the `faq_items` table.
    4.  Generate vector embeddings for *each* of the new Q&A pairs.

---

## Endpoint Consolidation & Behavior

### PDFs: `POST /ingest/pdf`

-   **Consolidation:** This single endpoint replaces `/ingest/file` and `/ingest/pdf_url`.
-   **Input:** Accepts `multipart/form-data` with either:
    -   A `file` part (for direct upload).
    -   A `url` part (for remote PDFs).
-   **Default:** Performs "Light Ingestion" on the PDF's refined text.
-   **With `?faq=true`:** Performs the full "FAQ Generation" pipeline.

### Google Sheets: `POST /ingest/sheet`

-   **Consolidation:** This single endpoint replaces `/ingest/sheet_faq`.
-   **Default:** Ingests the sheet as a generic, queryable table in the database. It then generates an embedding for each *row* of the table.
-   **With `?faq=true`:** Ingests a sheet specifically formatted with "Question" and "Answer" columns directly into the knowledge base, then generates an embedding for each Q&A pair.

### Web Pages: `POST /ingest/web`

-   **Default:** Performs "Light Ingestion" on the web page's cleaned markdown content.
-   **With `?faq=true`:** Performs the full "FAQ Generation" pipeline.

### Raw Text: `POST /ingest/text`

-   **Default:** Chunks the provided text and performs "Light Ingestion" for each individual chunk.
-   **With `?faq=true`:** Treats the entire text as a single document and performs the full "FAQ Generation" pipeline on it.

---

## Embedding Control

-   All endpoints listed above will accept an `embed` query parameter.
-   **Default:** `embed` is `true`.
-   **Opt-out:** Setting `?embed=false` will skip the vector embedding step for any ingestion workflow. This is useful for storing data without incurring embedding costs or processing time.

## The `/prompt` Handler

-   The primary user-facing `POST /prompt` endpoint remains unchanged.
-   It will continue to automatically ingest and generate FAQs from any URL detected in a user's query. This ensures a seamless, "magic" user experience without requiring the user to know about the new ingestion flags.