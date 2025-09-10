# Refactoring Plan: Agent-Based Context Retrieval for `/gen/text`

## 1. Objective

This document outlines a plan to refactor the `/gen/text` endpoint. The goal is to evolve it from a rigid, single-pipeline handler into an intelligent, agent-based system. This agent will analyze the user's `context_prompt` and dynamically choose the best internal tool to retrieve the most relevant context for content generation.

This change is motivated by the need to support more complex and nuanced context requests, such as "find the best stories using the knowledge graph," which the current Text-to-SQL-only approach cannot handle.

## 2. Current Implementation

The current `gen_text_handler` operates as a simple, linear pipeline:
1.  It receives a `context_prompt`.
2.  It unconditionally passes this prompt to a Text-to-SQL LLM call (`query_generation` task).
3.  It executes the resulting SQL query against a SQLite database.
4.  The raw JSON result from the query is used as context for the final `generation_prompt`.

This design is inflexible. It cannot perform semantic searches, query the knowledge graph, or re-rank results. It treats every context request as a task for a "Text-to-SQL Specialist," even when that is not the appropriate tool.

## 3. Proposed Refactoring: The Context-Fetching Agent

The handler will be re-architected to function as a two-stage agent.

### Stage 1: Tool Selection and Context Retrieval (LLM Call #1)

The first stage is responsible for understanding the user's `context_prompt` and using the best tool to fulfill it.

#### 3.1. Define Agent Tools

A set of tools will be programmatically defined and presented to the LLM in a system prompt. The LLM's task will be to choose a tool and provide the necessary arguments based on the user's prompt.

*   **Tool 1: `text_to_sql`**
    *   **Description for AI:** "Use this tool for precise, structured data retrieval. Best for prompts that mention specific columns (e.g., `rating`, `created_time`), tables, or ask for aggregations (e.g., `COUNT(*)`, `AVG(rating)`)."
    *   **Action:** Triggers the existing Text-to-SQL pipeline.
    *   **Example Trigger:** `"Use highest rating stories..."` -> AI chooses `text_to_sql`.

*   **Tool 2: `knowledge_search`**
    *   **Description for AI:** "Use this tool for broad, semantic, or conceptual searches. Best for prompts asking for the 'best' items, 'most relevant' stories, or about a general topic. This tool uses a powerful hybrid search that understands meaning and can re-rank results."
    *   **Action:** Triggers the `hybrid_search` function, the same one powering the `/search/knowledge` endpoint.
    *   **Example Trigger:** `"Use the graph to find the best love stories..."` -> AI chooses `knowledge_search`.

#### 3.2. Data Enrichment

A critical step will be added to enrich the data retrieved by the tools, particularly for the `knowledge_search` tool.

-   After `knowledge_search` returns a list of semantically relevant documents, the handler will perform a fast, secondary SQL query to fetch additional structured data for those specific documents (e.g., the `rating` column from the original table).
-   This ensures that the final context contains both semantic relevance and structured metrics, allowing for a truly "best of" result set.

### Stage 2: Final Content Generation (LLM Call #2)

The rich, high-quality context gathered and enriched in Stage 1 will be fed into the second LLM call. This call uses the user's original `generation_prompt` to craft the final output (e.g., the Pantip-style post), now informed by the best possible inspirational data.

## 4. Benefits of this Approach

*   **Increased Intelligence:** The endpoint will now understand the *intent* behind a `context_prompt` rather than just treating it as a SQL command.
*   **No API Changes:** This is a purely internal refactoring. The public API for `/gen/text` remains unchanged, with no need for new fields like `context_source`.
*   **Leverages Best Tools:** It allows the handler to use the most powerful and appropriate retrieval mechanism (`hybrid_search` for semantic queries, `SQL` for structured queries) for any given task.
*   **Superior Context:** The data enrichment step ensures the context provided for generation is more nuanced and complete, leading to higher-quality final outputs.

## 5. Refined Ingestion Strategy

To better align with the Single Responsibility Principle and create a more robust and understandable data pipeline, the ingestion process for structured data (like Firestore dumps) will be redesigned. The goal is to ensure that any structured data brought into the system is made available to *all* retrieval mechanisms (hybrid search and knowledge graph) through a clear, unified endpoint.

### 5.1. The `/ingest/firebase` Pipeline

This endpoint will be enhanced to become the primary entry point for structured data. It will orchestrate a multi-step process:

1.  **Dump Data**: It will connect to Firestore and dump the specified collection into a local SQLite table, overwriting any previous data from that collection to ensure a clean slate.
2.  **Build Metadata**: For each row in the newly dumped table, it will:
    *   Create a corresponding "shadow document" in the main `documents` table. The content of this document will be a concatenation of all text-based columns from the row.
    *   Run the AI-powered metadata extraction pipeline on this new document to generate and store keyphrases and entities. This is the crucial step that makes the structured data discoverable by hybrid search.
3.  **Build Graph (Optional)**: If the user provides a `use_graph=true` query parameter, the handler will then internally trigger the logic to build the Knowledge Graph from the new table.

### 5.2. The `/graph/build` Endpoint (Refactored)

This endpoint will be simplified and returned to its single, core responsibility:

*   It will **only** build or rebuild the in-memory Knowledge Graph from an *already existing* local SQLite table.
*   It will no longer be involved in document creation, metadata extraction, or any other ingestion-related tasks.

### 5.3. Benefits of this Refined Strategy

*   **Clear Separation of Concerns**: Ingestion and metadata generation are handled by ingestion endpoints. Graph building is handled by the graph endpoint. This is a cleaner, more maintainable design.
*   **Unified Ingestion Flow**: Users have a single, powerful command (`/ingest/firebase`) to make their structured data fully integrated and searchable across the entire application.
*   **Improved Testability**: Each component has a clearly defined job, making them easier to test in isolation.