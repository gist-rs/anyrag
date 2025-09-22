//! # Notion Ingestor Integration Tests

use anyhow::Result;
use anyrag::ingest::Ingestor;
use anyrag_notion::NotionIngestor;
use anyrag_test_utils::TestSetup;
use httpmock::{Method, MockServer};
use lazy_static::lazy_static;
use std::sync::Mutex;
use tracing::info;

use serde_json::json;
use std::env;
use turso::{params, Value as TursoValue};

lazy_static! {
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

#[tokio::test]
async fn test_notion_ingestion_workflow() -> Result<()> {
    let _guard = TEST_MUTEX.lock().unwrap();
    // --- 1. Arrange & Setup ---
    let setup = TestSetup::new().await?;
    let mock_server = MockServer::start();
    let owner_id = "notion-ingest-user-001";

    let db_id = "mock-db-id-12345";
    let data_source_id = "mock-ds-id-67890";

    // Set mock credentials for the test
    env::set_var(
        "NOTION_API_BASE_URL_OVERRIDE_FOR_TESTING",
        mock_server.base_url(),
    );
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
            .path(format!("/v1/databases/{}", db_id));
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
                            "end": "2024-08-01T12:00:00.000Z"
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
            .path(format!("/v1/data_sources/{}/query", data_source_id));
        then.status(200)
            .header("Content-Type", "application/json")
            .json_body(query_response);
    });

    // --- 3. Act: Ingest the Notion Database ---
    let ingestor = NotionIngestor::new(&setup.db);
    let source = json!({ "database_id": db_id }).to_string();

    let result = ingestor.ingest(&source, Some(owner_id)).await?;

    // --- 4. Assert Ingestion Result & Database State ---
    assert_eq!(
        result.documents_added, 4,
        "Expected 4 rows after hourly expansion (3 from one page, 1 from another)"
    );
    let table_name = &result.document_ids[0];
    let expected_table_name = format!(
        "notion_{:x}",
        md5::compute(format!("{}::{}", db_id, data_source_id))
    );
    assert_eq!(table_name, &expected_table_name);

    let conn = setup.db.connect()?;

    let mut stmt = conn
        .prepare(&format!(
            "SELECT `Task`, `Status`, `busy_date`, `busy_time` FROM `{}` ORDER BY `Task`, `busy_time`",
            table_name
        ))
        .await?;
    let mut rows = stmt.query(params![]).await?;

    // First result: "Review PR" (no date expansion)
    let row1 = rows.next().await?.expect("Expected row 1");
    assert_eq!(row1.get::<String>(0)?, "Review PR");
    assert_eq!(row1.get::<String>(1)?, "Done");
    assert_eq!(row1.get_value(2)?, TursoValue::Null); // busy_date
    assert_eq!(row1.get_value(3)?, TursoValue::Null); // busy_time

    // Second result: "Write integration test" (Hour 1 of expansion)
    let row2 = rows.next().await?.expect("Expected row 2");
    assert_eq!(row2.get::<String>(0)?, "Write integration test");
    assert_eq!(row2.get::<String>(1)?, "In Progress");
    assert_eq!(row2.get::<String>(2)?, "2024-08-01"); // busy_date
    assert_eq!(row2.get::<String>(3)?, "10:00:00"); // busy_time

    // Third result: "Write integration test" (Hour 2 of expansion)
    let row3 = rows.next().await?.expect("Expected row 3");
    assert_eq!(row3.get::<String>(0)?, "Write integration test");
    assert_eq!(row3.get::<String>(1)?, "In Progress");
    assert_eq!(row3.get::<String>(2)?, "2024-08-01"); // busy_date
    assert_eq!(row3.get::<String>(3)?, "11:00:00"); // busy_time

    // Fourth result: "Write integration test" (Hour 3 of expansion)
    let row4 = rows.next().await?.expect("Expected row 4");
    assert_eq!(row4.get::<String>(0)?, "Write integration test");
    assert_eq!(row4.get::<String>(1)?, "In Progress");
    assert_eq!(row4.get::<String>(2)?, "2024-08-01"); // busy_date
    assert_eq!(row4.get::<String>(3)?, "12:00:00"); // busy_time

    assert!(
        rows.next().await?.is_none(),
        "Found more rows than expected (4)"
    );

    // --- 5. Assert Mocks Were Called ---
    db_details_mock.assert();
    query_mock.assert();

    // Cleanup env var
    env::remove_var("NOTION_API_BASE_URL_OVERRIDE_FOR_TESTING");

    Ok(())
}

#[tokio::test]
async fn test_notion_ingestion_hourly_expansion() -> Result<()> {
    let _guard = TEST_MUTEX.lock().unwrap();
    // --- 1. Arrange & Setup ---
    let setup = TestSetup::new().await?;
    let mock_server = MockServer::start();

    // Set env vars immediately after server starts to ensure the http client uses the mock URL.
    env::set_var(
        "NOTION_API_BASE_URL_OVERRIDE_FOR_TESTING",
        mock_server.base_url(),
    );
    env::set_var("NOTION_TOKEN", "test_token");
    env::set_var("NOTION_VERSION", "2022-06-28");

    let owner_id = "notion-ingest-user-002";

    let db_id = "mock-db-id-hourly";
    let data_source_id = "mock-ds-id-hourly";
    // --- 2. Mock Notion API Responses ---
    let db_details_response = json!({
        "id": db_id,
        "data_sources": [{ "id": data_source_id, "name": "Mock DB Hourly" }]
    });

    let db_details_mock = mock_server.mock(|when, then| {
        when.method(Method::GET)
            .path(format!("/v1/databases/{}", db_id));
        then.status(200)
            .header("Content-Type", "application/json")
            .json_body(db_details_response);
    });

    let query_response = json!({
        "object": "list",
        "results": [
            {
                "object": "page",
                "id": "page_hourly",
                "properties": {
                    "Event": {
                        "id": "title",
                        "type": "title",
                        "title": [{ "plain_text": "Morning Session" }]
                    },
                    "Schedule": {
                        "id": "date1",
                        "type": "date",
                        "date": {
                            "start": "2025-08-01T09:00:00.000Z",
                            "end": "2025-08-01T12:00:00.000Z"
                        }
                    }
                }
            }
        ],
        "has_more": false,
        "next_cursor": null
    });

    let query_mock = mock_server.mock(|when, then| {
        when.method(Method::POST)
            .path(format!("/v1/data_sources/{}/query", data_source_id));
        then.status(200)
            .header("Content-Type", "application/json")
            .json_body(query_response);
    });

    // --- 3. Act ---
    let ingestor = NotionIngestor::new(&setup.db);
    let source = json!({ "database_id": db_id }).to_string();
    let result = ingestor.ingest(&source, Some(owner_id)).await?;

    info!(
        "Hourly Expansion Result: documents_added={}, source='{}', document_ids='{:?}'",
        result.documents_added, result.source, result.document_ids
    );

    // --- 4. Assert ---
    assert_eq!(
        result.documents_added, 4,
        "Expected 4 rows for 4 hours (9, 10, 11, 12)"
    );
    let table_name = &result.document_ids[0];
    info!(?table_name, "Table name for hourly expansion");

    let conn = setup.db.connect()?;
    let mut stmt = conn
        .prepare(&format!(
            "SELECT `Event`, `busy_date`, `busy_time` FROM `{}` ORDER BY `busy_time`",
            table_name
        ))
        .await?;
    let mut rows = stmt.query(params![]).await?;

    // Assert row for 09:00
    let row1 = rows.next().await?.expect("Expected row for 09:00");
    info!(?row1, "Row 1");
    assert_eq!(row1.get::<String>(0)?, "Morning Session");
    assert_eq!(row1.get::<String>(1)?, "2025-08-01");
    assert_eq!(row1.get::<String>(2)?, "09:00:00");

    // Assert row for 10:00
    let row2 = rows.next().await?.expect("Expected row for 10:00");
    info!(?row2, "Row 2");
    assert_eq!(row2.get::<String>(0)?, "Morning Session");
    assert_eq!(row2.get::<String>(2)?, "10:00:00");

    // Assert row for 11:00
    let row3 = rows.next().await?.expect("Expected row for 11:00");
    info!(?row3, "Row 3");
    assert_eq!(row3.get::<String>(0)?, "Morning Session");
    assert_eq!(row3.get::<String>(2)?, "11:00:00");

    // Assert row for 12:00
    let row4 = rows.next().await?.expect("Expected row for 12:00");
    info!(?row4, "Row 4");
    assert_eq!(row4.get::<String>(0)?, "Morning Session");
    assert_eq!(row4.get::<String>(2)?, "12:00:00");

    assert!(rows.next().await?.is_none(), "Expected exactly 4 rows");

    // --- 5. Cleanup ---
    db_details_mock.assert();
    query_mock.assert();
    env::remove_var("NOTION_API_BASE_URL_OVERRIDE_FOR_TESTING");

    Ok(())
}
