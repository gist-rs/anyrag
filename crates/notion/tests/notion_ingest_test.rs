//! # Notion Ingestor Integration Tests

use anyhow::Result;
use anyrag::ingest::Ingestor;
use anyrag_notion::NotionIngestor;
use anyrag_test_utils::TestSetup;
use httpmock::{Method, MockServer};
use serde_json::json;
use std::env;
use turso::{params, Value as TursoValue};

#[tokio::test]
async fn test_notion_ingestion_workflow() -> Result<()> {
    // --- 1. Arrange & Setup ---
    let setup = TestSetup::new().await?;
    let mock_server = MockServer::start();
    let owner_id = "notion-ingest-user-001";

    let db_id = "mock-db-id-12345";
    let data_source_id = "mock-ds-id-67890";

    // Set mock credentials for the test
    env::set_var("NOTION_TOKEN", "test_token");
    env::set_var("NOTION_VERSION", "2022-06-28");

    // --- 2. Mock Notion API Responses ---

    // A. Mock the response for getting database details
    let db_details_response = json!({
        "id": db_id,
        "data_sources": [{ "id": data_source_id, "name": "Mock DB" }]
    });

    let db_details_mock = mock_server.mock(|when, then| {
        when.method(Method::GET)
            .path(format!("/v1/databases/{db_id}"));
        then.status(200)
            .header("Content-Type", "application/json")
            .json_body(db_details_response);
    });

    // B. Mock the response for querying the data source
    let query_response = json!({
        "object": "list",
        "results": [
            {
                "object": "page",
                "id": "page1",
                "properties": {
                    "Task": {
                        "id": "title",
                        "type": "title",
                        "title": [{ "plain_text": "Write integration test" }]
                    },
                    "Status": {
                        "id": "status1",
                        "type": "rich_text",
                        "rich_text": [{ "plain_text": "In Progress" }]
                    },
                    "Timeline": {
                        "id": "date1",
                        "type": "date",
                        "date": {
                            "start": "2024-08-01T10:00:00.000Z",
                            "end": "2024-08-02T10:00:00.000Z"
                        }
                    }
                }
            },
            {
                "object": "page",
                "id": "page2",
                "properties": {
                    "Task": {
                        "id": "title",
                        "type": "title",
                        "title": [{ "plain_text": "Review PR" }]
                    },
                    "Status": {
                        "id": "status2",
                        "type": "rich_text",
                        "rich_text": [{ "plain_text": "Done" }]
                    },
                    "Timeline": {
                        "id": "date2",
                        "type": "date",
                        "date": null
                    }
                }
            }
        ],
        "has_more": false,
        "next_cursor": null
    });

    let query_mock = mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(format!("/v1/data_sources/{data_source_id}/query"));
        then.status(200)
            .header("Content-Type", "application/json")
            .json_body(query_response);
    });

    // --- 3. Act: Ingest the Notion Database ---
    // Override the Notion API base URL to point to our mock server
    env::set_var(
        "NOTION_API_BASE_URL_OVERRIDE_FOR_TESTING",
        mock_server.base_url(),
    );

    let ingestor = NotionIngestor::new(&setup.db);
    let source = json!({ "database_id": db_id }).to_string();

    let result = ingestor.ingest(&source, Some(owner_id)).await?;

    // --- 4. Assert Ingestion Result & Database State ---
    assert_eq!(
        result.documents_added, 3,
        "Expected 3 rows after expansion (2 from one page, 1 from another)"
    );
    let table_name = &result.document_ids[0];
    let expected_table_name = format!(
        "notion_{:x}",
        md5::compute(format!("{db_id}::{data_source_id}"))
    );
    assert_eq!(table_name, &expected_table_name);

    let conn = setup.db.connect()?;

    // A. Query the created table and verify its contents.
    // The order of columns might vary, so we query all and check values.
    let mut stmt = conn
        .prepare(&format!(
            "SELECT `Task`, `Status`, `busy_date`, `busy_time` FROM `{table_name}` ORDER BY `Task`, `busy_date`"
        ))
        .await?;
    let mut rows = stmt.query(params![]).await?;

    // First result: "Review PR" (no date expansion)
    let row1 = rows.next().await?.expect("Expected row 1");
    assert_eq!(row1.get::<String>(0)?, "Review PR");
    assert_eq!(row1.get::<String>(1)?, "Done");
    assert_eq!(row1.get_value(2)?, TursoValue::Null); // busy_date
    assert_eq!(row1.get_value(3)?, TursoValue::Null); // busy_time

    // Second result: "Write integration test" (Day 1 of expansion)
    let row2 = rows.next().await?.expect("Expected row 2");
    assert_eq!(row2.get::<String>(0)?, "Write integration test");
    assert_eq!(row2.get::<String>(1)?, "In Progress");
    assert_eq!(row2.get::<String>(2)?, "2024-08-01"); // busy_date
    assert_eq!(row2.get::<String>(3)?, "10:00:00"); // busy_time

    // Third result: "Write integration test" (Day 2 of expansion)
    let row3 = rows.next().await?.expect("Expected row 3");
    assert_eq!(row3.get::<String>(0)?, "Write integration test");
    assert_eq!(row3.get::<String>(1)?, "In Progress");
    assert_eq!(row3.get::<String>(2)?, "2024-08-02"); // busy_date
    assert_eq!(row3.get::<String>(3)?, "10:00:00"); // busy_time

    assert!(
        rows.next().await?.is_none(),
        "Found more rows than expected (3)"
    );

    // --- 5. Assert Mocks Were Called ---
    db_details_mock.assert();
    query_mock.assert();

    // Cleanup env var
    env::remove_var("NOTION_API_BASE_URL_OVERRIDE_FOR_TESTING");

    Ok(())
}
