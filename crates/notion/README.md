# `anyrag-notion`: Notion Ingestion Plugin

This crate provides the logic for ingesting data from a Notion database as a self-contained plugin for the `anyrag` ecosystem. It implements the `Ingestor` trait from the core `anyrag` library.

The primary goal of this plugin is to connect to a Notion database, extract its rows (pages), and transform them into a structured format within a dedicated, file-based SQLite database. This process makes Notion data available for powerful natural language querying through `anyrag`'s core NL-to-SQL capabilities.

## Features

-   **Dynamic Schema Generation**: The schema of the target SQLite table is created dynamically to match the properties of your Notion database.
-   **Date Range Expansion**: A key feature is the ability to expand Notion `date` properties that have a start and end time. Each hour within the specified range is expanded into a separate row in the database, creating granular, queryable data.
-   **Isolated, File-Based Storage**: Each Notion data source is ingested into its own unique SQLite file (`.db`). The filename is deterministically generated from the Notion `database_id` and the discovered `data_source_id`, ensuring no data collisions.
-   **Clear and Informative Output**: Returns detailed metadata about the ingestion process, including the discovered `data_source_id` and the final database filename.

## Example: End-to-End Ingestion and Search

The crate includes a powerful example that demonstrates the full workflow: ingesting a real Notion database and then immediately using `anyrag`'s AI capabilities to ask a question about the data in natural language.

### 1. Prerequisites

Create a `.env` file in the root of the `anyrag` workspace and add the following variables. You will also need credentials for an AI provider (like a local Ollama instance).

```env
# Your Notion integration token
NOTION_TOKEN="secret_..."
# The Notion API version
NOTION_VERSION="2022-06-28"
# The ID of the Notion database you want to test with
NOTION_TEST_DB_ID="your_notion_database_id"

# --- AI Provider Settings ---
# Can be "local" or "gemini"
AI_PROVIDER="local"
# URL for your local LLM (e.g., Ollama)
LOCAL_AI_API_URL="http://localhost:11434/v1/chat/completions"
# The specific model to use for NL-to-SQL
AI_MODEL="llama3"
# (Optional) API key if your local provider needs one
AI_API_KEY=""
```

### 2. Run the Example

From the `anyrag` workspace root, execute the following command:

```sh
cargo run -p anyrag-notion --example ingest_notion
```

### 3. Expected Output

The example will first ingest the data from your Notion database, creating a new `.db` file in the `db/` directory. Then, it will use the `PromptClient` to ask "Who is available today?" against that new database.

The output will look something like this:

```
--- Starting Notion ingestion ---

--- Ingestion Complete ---
Source Database ID: 276fdc986cf080418e59df98022eee89
Discovered Data Source ID: 276fdc98-6cf0-806b-bb82-000baa57dddb
Generated Table Name: notion_8e65daa34069989fd62968b91105e761
Data saved to database file: db/notion_8e65daa34069989fd62968b91105e761.db
Documents (rows) added: 723

--- Verifying with NL-to-SQL Search ---
# CONTEXT
- # TODAY: 2025-09-23T...
# QUERY
- Who is available today?

# RESPONSE
- AI Generated Answer:
Based on the data, the following people are available today:
  - テスト講師
  - 犀川巧

- Generated SQL:
SELECT DISTINCT `講師名` FROM `notion_8e65daa34069989fd62968b91105e761`
EXCEPT
SELECT `講師名` FROM `notion_8e65daa34069989fd62968b91105e761` WHERE `busy_date` = '2025-09-23'
```
