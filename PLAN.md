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

## 6. Phase 2: User-Directed Retrieval and Enhanced Query Analysis

### 6.1. Objective

Based on feedback, the initial agent implementation, while functional, can misinterpret the user's core intent by overly refining the `context_prompt`. This next phase of refactoring has two primary goals:

1.  **Provide Explicit User Control**: Introduce a set of flags in the `/gen/text` API to allow users to bypass the agent's tool selection and directly control the retrieval methods used.
2.  **Enhance Implicit Analysis**: For users who do not provide explicit flags, significantly improve the AI-driven analysis of the `context_prompt` to better distinguish between the "search query" and the "generative intent."

### 6.2. API and Payload Changes

The `GenTextRequest` payload will be extended to include several new optional fields. This gives users a "control panel" to fine-tune the context retrieval process.

The `GenTextRequest` struct in `anyrag/crates/server/src/handlers/generation_handlers.rs` will be updated:

```rust
// The New GenTextRequest
#[derive(Deserialize, Debug)]
pub struct GenTextRequest {
    // Existing fields
    #[serde(default)]
    pub db: Option<String>,
    pub generation_prompt: String,
    #[serde(default)]
    pub context_prompt: Option<String>,
    #[serde(default)]
    pub model: Option<String>,

    // New Control Flags
    #[serde(default)]
    pub use_sql: bool, // Default: true if `db` is present, otherwise false
    #[serde(default)]
    pub use_knowledge_search: bool, // Default: true if no `db` is present
    #[serde(default = "default_true")]
    pub use_keyword_search: bool, // Default: true (within knowledge search)
    #[serde(default = "default_true")]
    pub use_vector_search: bool, // Default: true (within knowledge search)
    pub rerank_limit: Option<u32>, // Default: 10
}
```

### 6.3. Agent and Handler Logic Refinement

The `gen_text_handler` will be updated to respect these new flags.

1.  **Direct Routing (Explicit Control)**: If `use_sql: true` or `use_knowledge_search: true` is explicitly set in the request, the agent's tool-selection AI call will be **bypassed entirely**. The handler will route the `context_prompt` directly to the specified tool. This provides a fast and predictable path for users who know what they want.

2.  **Smarter Analysis (Implicit Control)**: If no explicit routing flags are set, the agent will proceed, but with a new, more sophisticated analysis step.

### 6.4. New AI Step: Query Deconstruction

The core of the improved implicit analysis is a new AI prompt that deconstructs the `context_prompt` instead of just refining it.

**Old Behavior:**
`"สร้างเรื่องเกี่ยวกับความรักสามเส้า"` -> (is refined to) -> `"ความรักสามเส้า"` -> (loses the "create a story" intent).

**New Behavior:**
The handler will send the `context_prompt` to a new AI task with the following instructions:

**System Prompt:**
> You are a query analyst. Deconstruct the user's request into two parts: a concise `search_query` for finding relevant data, and the full `generative_intent` which is the user's original goal.

**User Prompt:**
> User's Request: `"สร้างเรื่องเกี่ยวกับความรักสามเส้า"`

**Expected AI Response (JSON):**
```json
{
  "search_query": "เรื่องราวความรัก, รักสามเส้า",
  "generative_intent": "สร้างเรื่องเกี่ยวกับความรักสามเส้า"
}
```

The handler will then use these two outputs intelligently:
*   The `search_query` will be sent to the chosen retrieval tool (`knowledge_search` or `text_to_sql`).
*   The `generative_intent` will be preserved and used alongside the main `generation_prompt` in the final content generation step, ensuring the original context is never lost.

### 6.5. Search Pipeline Enhancements

The underlying `hybrid_search` function will be updated to support the new controls and improve robustness.

1.  **Parallel Execution**: Keyword and vector searches will be executed in parallel using `tokio::join!` for improved performance.
2.  **Soft Failure**: If one search method fails (e.g., vector search returns an error), the error will be logged, but the pipeline will continue with the results from the successful methods. This prevents a single point of failure from stopping the entire request.
3.  **Configurable Limits**: The `rerank_limit` from the API payload will be passed down to the final re-ranking and truncation step, allowing users to control the number of candidates they want to consider.

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

## 7. Phase 3: Agent Tool Expansion and Autonomous Operation

### 7.1. Objective

To evolve the agent from a simple tool selector into a more autonomous system capable of answering a wider range of questions and performing multi-step tasks by expanding its set of available tools.

### 7.2. New Agent Tools

The agent's system prompt will be updated to include new tools that broaden its capabilities beyond database lookups.

*   **Tool 3: `web_search`**
    *   **Description for AI:** "Use this tool for questions about recent events, general knowledge, or topics not covered in the local knowledge base. Best for queries that require up-to-date, real-world information."
    *   **Action:** Triggers a call to an external web search API (e.g., Jina Search) to retrieve snippets of relevant information.

*   **Tool 4: `file_system_read`**
    *   **Description for AI:** "Use this tool to read the content of a local file when the user provides a specific, relative file path in their prompt."
    *   **Action:** Reads a file from a sandboxed, pre-approved directory within the project and returns its content as context.

### 7.3. Chained Tool Use (Autonomous Operation)

The agent's core logic will be enhanced to support multi-step reasoning. Instead of selecting a single tool and finishing, the agent will be able to chain tool calls together.

*   **Prompt Engineering**: The agent's main system prompt will be updated to encourage multi-step thinking. It will be instructed to formulate a plan, execute a tool, observe the result, and decide on the next step until the user's final goal is achieved.
*   **Example Flow**: A user prompt like "Search the web for the latest Rust release notes and summarize the key features mentioned in `docs/CHANGELOG.md`" would trigger the following chain:
    1.  **Agent Plan**: (1) Use `web_search` to find the latest Rust release info. (2) Use `file_system_read` to get the content of `docs/CHANGELOG.md`. (3) Synthesize the final summary.
    2.  **Tool Call 1**: `web_search(query="latest Rust release notes")`
    3.  **Tool Call 2**: `file_system_read(path="docs/CHANGELOG.md")`
    4.  **Final Generation**: The agent combines the results from both tools to generate the final, comprehensive answer.

### 7.4. Benefits of this Approach

*   **Wider Range of Capabilities**: The agent can now answer questions about current events and read local project files.
*   **Complex Problem Solving**: Enables the agent to tackle multi-step tasks that require gathering information from multiple sources.
*   **Increased Autonomy**: Reduces the need for the user to manually break down a complex request into smaller, manageable prompts.