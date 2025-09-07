# Feature Plan: Enhanced Querying and Generation API

This plan outlines three new ways to interact with the data stored in the `anyrag` system, catering to different needs from direct data access to advanced, context-aware content generation.

### 1. Direct, Parameter-based Querying

To address the need for a simple, direct way to list table contents, a new, powerful endpoint will be created. This is perfect for programmatic access or for when you know exactly what you want without needing natural language translation.

*   **Endpoint:** `POST /db/query`
*   **Functionality:** This endpoint will accept a raw, read-only SQL query and execute it directly against the appropriate project's database. This gives precise control over data retrieval.
*   **Request Body Example:**
    ```json
    {
      "project_id": "kratooded",
      "query": "SELECT _id, title, rating FROM pantip_topics_samples WHERE rating >= 3 ORDER BY rating DESC LIMIT 10"
    }
    ```
*   **Security:** The handler will validate that the query is read-only (i.e., it must start with `SELECT`).

### 2. Shorthand Prompt Aliases

For quick, interactive exploration, the existing `/prompt` endpoint will be enhanced to recognize and translate command-like prompts.

*   **Endpoint:** `POST /prompt` (Existing)
*   **Shorthand Example:**
    ```json
    {
      "project_id": "kratooded",
      "prompt": "ls pantip_topics_samples limit=20"
    }
    ```
*   **Internal Logic:** The server will detect this pattern and internally convert it into a full request for the Text-to-SQL engine, effectively asking it to "List the first 20 rows from the `pantip_topics_samples` table." This reuses the existing powerful NL-to-SQL pipeline.

### 3. Advanced Content Generation with "Best Search"

This addresses the most sophisticated use case: generating new content based on data retrieved from the database using the system's most powerful search capabilities.

*   **Endpoint:** `POST /gen/text`
*   **"Best Search" Definition:** This refers to the full RAG pipeline exposed via `/search/knowledge`. It is a multi-stage process that uses **hybrid search** (metadata filtering + vector search) and automatically integrates context from the **Knowledge Graph** to find the most relevant information.
*   **Request Body Example:**
    ```json
    {
      "project_id": "kratooded",
      "generation_prompt": "Write a short, romantic story in the style of a modern Thai drama.",
      "context_prompt": "Use themes and characters from the highest-rated stories about 'love' (ความรัก) in the 'pantip_topics_samples' table as inspiration."
    }
    ```
*   **Workflow:**
    1.  **Context Retrieval via "Best Search":** The server executes a "Best Search" by internally calling the `/search/knowledge` logic with the `context_prompt`. This retrieves the most relevant documents and facts.
    2.  **Content Generation:** The server then combines the rich context retrieved from the "Best Search" with the original `generation_prompt` and sends it to the AI to generate the final text.
    3.  **Error Handling:** If the "Best Search" returns no results (e.g., the knowledge base is not prepared), the generation step will fail with a clear error message indicating that the required context could not be found.

### 4. Data Ingestion and Knowledge Graph Pipeline

To support the advanced generation capabilities, the following endpoints will be created to manage the data lifecycle from the remote source (Firestore) to the in-memory Knowledge Graph.

*   **New Endpoint: `POST /ingest/firebase`**
    *   **Functionality:** Triggers a server-side dump of a Firestore collection into the corresponding project's local SQLite database (`db/<project_id>.db`). This brings the core functionality of the `anyrag-cli` directly into the server, enabling automated data synchronization.
    *   **Request Body:** Similar to the CLI, it will accept `project_id`, `collection`, and options like `incremental` and `fields`.

*   **New Endpoint: `POST /graph/build`**
    *   **Functionality:** Reads data from a specified table within a local SQLite database and uses it to construct or update the in-memory Knowledge Graph. This decouples graph building from data fetching, allowing the graph to be rebuilt from local data at any time without needing to contact Firestore.
    *   **Request Body:** Will specify the `project_id` (to find the DB) and the `table_name` to process.

#### End-to-End Generation Workflow

The complete "dump, make graph, then gen story" workflow will now be a clear sequence of three API calls:

1.  **`POST /ingest/firebase`**: Dumps the latest `pantip_topics_samples` from Firestore to the local SQLite DB.
2.  **`POST /graph/build`**: Processes the `pantip_topics_samples` table, populating the in-memory Knowledge Graph with relevant, time-sensitive facts.
3.  **`POST /gen/text`**: Executes the generation request, which now uses a fully populated knowledge base and graph for context.
