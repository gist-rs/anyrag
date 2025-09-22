# Notion Integration Plan

This document outlines the plan to integrate Notion as a data source for ingestion into the `anyrag` ecosystem. The primary goal is to query Notion databases, extract their content, and store it in a structured format within a SQLite database for later retrieval and analysis.

## 1. API Endpoints & Authentication

We will interact with the Notion API using the following endpoints:

### A. Retrieve a Database

This step is necessary to get metadata about the database, including its structure and associated `data_source` IDs.

- **Request:**
  ```sh
  curl --request GET \
      --url https://api.notion.com/v1/databases/${DATABASE_ID} \
      -H 'Authorization: Bearer ${NOTION_TOKEN}' \
      -H 'Notion-Version: ${NOTION_VERSION}'
  ```

- **Example Response:**
  ```json
  {
    "object": "database",
    "id": "276fdc98-6cf0-8041-8e59-df98022eee89",
    "title": [...],
    "data_sources": [
      {
        "id": "276fdc98-6cf0-806b-bb82-000baa57dddb",
        "name": "New database"
      }
    ],
    ...
  }
  ```

### B. Query a Data Source

Using the `data_source_id` from the previous step, we will query for all the pages (rows) within that data source.

- **Request:**
  ```sh
  curl -X POST 'https://api.notion.com/v1/data_sources/${DATA_SOURCE_ID}/query' \
      -H 'Authorization: Bearer ${NOTION_TOKEN}' \
      -H 'Notion-Version: ${NOTION_VERSION}' \
      -H "Content-Type: application/json"
  ```

- **Example Response:**
  The response is a list of page objects, each representing a row in the database.
  ```json
  {
    "object": "list",
    "results": [
      {
        "object": "page",
        "id": "276fdc98-6cf0-801b-92c9-dee72c19eb1d",
        "properties": {
          "対応可能日程 (2)": {
            "id": "ROW%3F",
            "type": "rich_text",
            "rich_text": [ ... ]
          },
          "講師名": {
            "id": "title",
            "type": "title",
            "title": [ ... ]
          }
        },
        ...
      }
    ],
    ...
  }
  ```

## 2. Data Extraction and Transformation

The core of the ingestion logic will involve processing the `results` array from the query endpoint.

1.  **Iterate Pages:** Process each page object in the `results` array.
2.  **Extract Properties:** For each page, iterate through its `properties`. The value of each property will be extracted. For `rich_text` and `title` types, we will concatenate the `plain_text` fields.
3.  **Handle Date Ranges:** A special transformation will be applied to properties of type `date`. A Notion property like:
    ```json
    {
      "id": "...",
      "type": "date",
      "date": {
        "start": "2025-09-18T10:00:00.000+09:00",
        "end": "2025-09-30T10:00:00.000+09:00",
        "time_zone": null
      }
    }
    ```
    This will be expanded into multiple rows in the target SQLite table. Each day within the range will generate a new row. The other property values from the original Notion row will be duplicated for each generated row.
    -   A `busy_date` column will store the date (e.g., `2025-09-18`).
    -   A `busy_time` column will store the start time (e.g., `10:00:00`).

## 3. Database Schema and Naming

-   **Database Name:** A unique database file will be created for each Notion data source. The name will be derived using the formula: `notion_{md5(database_id::datasource_id)}`.
-   **Table Schema:**
    -   A table will be created within the database. The table name could be derived from the Notion database `title`.
    -   The columns of the table will dynamically match the properties of the Notion database. The column name will be the property name (e.g., `講師名`).
    -   For date range properties, instead of a single column, two columns will be created: `busy_date` and `busy_time`.

## 4. Implementation Plan

-   Create a new crate `anyrag-notion` similar to `anyrag-sheets`.
-   Implement `NotionIngestor` which will implement the `Ingestor` trait.
-   The `ingest` method will accept a JSON string specifying the `database_id`, e.g., `{"database_id": "276fdc98-6cf0-8041-8e59-df98022eee89"}`.
-   Use `reqwest` for making HTTP requests to the Notion API.
-   Define Rust structs using `serde` to deserialize the JSON responses from Notion for type safety.
-   Use the `turso` crate to create and populate the SQLite database.
-   Add comprehensive tests using `httpmock` to mock the Notion API responses and verify the database contents.