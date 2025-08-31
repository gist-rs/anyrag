# IMMEDIATE ACTION PLAN: Hybrid Search & DB Refactor

**Objective**: To re-architect the core application by decoupling data storage from search indexes and implementing a high-performance, multi-stage search pipeline. This plan introduces **Hybrid Metadata Extraction** (Entities + Keyphrases) as the central mechanism for fast and relevant information retrieval.

This is the **current, focused task** for the development team.

## 1. The Core Problem & Solution

-   **Problem**: Storing large content blobs alongside search embeddings is inefficient. Simple tag-based search is too restrictive and misses the thematic essence of a document.
-   **Solution**: We will normalize the database schema to separate content from search indexes. We will then implement a sophisticated search flow that uses a hybrid metadata index (containing both specific entities and broad keyphrases) for a powerful pre-filtering step.

## 2. New Database Schema

-   **`documents`**: The central source of truth for all ingested content (`id`, `owner_id`, `source_url`, `title`, `content`, `created_at`, `expires_at`).
-   **`document_embeddings`**: A lean table for fast vector search (`document_id`, `model_name`, `embedding`).
-   **`faq_items`**: Stores structured Q&A pairs extracted from documents (`id`, `document_id`, `owner_id`, `question`, `answer`).
-   **`content_metadata`** (formerly `content_tags`): A heavily indexed table for fast, hybrid metadata filtering.
    ```sql
    CREATE TABLE IF NOT EXISTS content_metadata (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        document_id TEXT NOT NULL,
        owner_id TEXT, -- Denormalized for efficient filtering
        metadata_type TEXT NOT NULL, -- 'ENTITY', 'KEYPHRASE'
        metadata_subtype TEXT, -- e.g., 'PERSON', 'PRODUCT', 'CONCEPT'
        metadata_value TEXT NOT NULL,
        FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
    );
    CREATE INDEX idx_metadata_value ON content_metadata(metadata_value);
    CREATE INDEX idx_metadata_owner_id ON content_metadata(owner_id);
    ```

## 3. New Search & Ingestion Architecture

### 3.1. Hybrid Metadata Extraction (Ingestion)

During ingestion (as a background job), we will use an LLM to extract a rich set of metadata from the source content.

-   **Process**: A sophisticated prompt will instruct the LLM to act as a document indexer and extract two distinct types of metadata.
-   **Prompt Draft**:
    > You are an expert document analyst. Analyze the following text and extract **Entities** (specific proper nouns like people, products, organizations) and **Keyphrases** (the 5-10 most important thematic concepts). Return a single JSON array of objects, each with a `type` ('ENTITY' or 'KEYPHRASE'), a `subtype` (e.g., 'PERSON', 'CONCEPT'), and a `value`.
    >
    > **Text to Analyze**: {markdown_content}
    >
    > **Your JSON Output**:
-   **Storage**: The extracted metadata will be stored in the `content_metadata` table.

### 3.2. Multi-Stage Search Flow (Retrieval)

The RAG endpoint (`/search/knowledge`) will be re-architected to follow this high-performance flow:

1.  **Query Analysis**: An LLM extracts key **Entities** and **Keyphrases** from the user's query.
2.  **Metadata Pre-Filtering (Stage 1)**: A fast, secure SQL query runs on the `content_metadata` table to find `document_id`s that match the extracted metadata and the user's access permissions. This yields a small set of highly relevant candidate documents.
3.  **Vector Re-Ranking (Stage 2)**: A vector search is performed on the `document_embeddings` table, but is restricted **only** to the candidate `document_id`s from Stage 1 (`WHERE document_id IN (...)`). This returns a final, ranked list of the most semantically relevant `document_id`s.
4.  **Content Retrieval**: The application fetches the full content for the final `document_id`s from the `documents` or `faq_items` tables.
5.  **LLM Synthesis**: The retrieved content is used as context for the final LLM call to synthesize the answer.

## 4. Implementation Roadmap

1.  **[DB Schema]** Implement the new table creation logic for `documents`, `document_embeddings`, `faq_items`, and `content_metadata`.
2.  **[Ingestion Logic]** Refactor the ingestion pipeline to:
    -   Write to the new normalized schema (`documents`, `faq_items`).
    -   Make the new **Hybrid Metadata Extraction** LLM call.
    -   Populate the `content_metadata` table.
3.  **[Embedding Logic]** Update embedding processes to populate the new `document_embeddings` table.
4.  **[Search Logic]** Rewrite the `knowledge_search_handler` to execute the full **Multi-Stage Search Flow**.
5.  **[API]** Create the new `POST /search/metadata` endpoint for direct metadata lookups.
6.  **[Testing]** Update all relevant integration tests to use and validate the new architecture.

## 5. Success Criteria

-   All existing functionality is preserved.
-   All new and existing tests pass.
-   The database schema is fully migrated to the new, normalized structure.
-   Search relevance and performance are measurably improved.