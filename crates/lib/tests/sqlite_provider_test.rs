//! # SQLite Provider Tests
//!
//! This file contains tests specifically for the `SqliteProvider`.
//! These tests verify its core functionality, such as connecting to a database,
//! executing queries, and handling data correctly, ensuring that the provider
//! is a reliable storage backend for the `anyrag` library.
//!
//! Each test uses an in-memory database to ensure they are fast and isolated
//! from one another, with no need for file system cleanup.

// This declaration makes the `common` module available to the tests in this file.
mod common;

use crate::common::setup_tracing;
use anyrag::providers::db::sqlite::SqliteProvider;
use anyrag::providers::db::storage::Storage;
use anyrag::PromptError;
use chrono::Utc;
use serde_json::json;

/// This test is adapted from the official Turso repository to verify basic DB operations.
/// It confirms that we can connect, create a table, insert data, and query it back.
#[tokio::test]
async fn test_sqlite_provider_basic_crud() {
    setup_tracing();

    // 1. Setup: Create a new in-memory SQLite provider.
    // Using ":memory:" is fast and ensures the test is isolated.
    let provider = SqliteProvider::new(":memory:")
        .await
        .expect("Failed to create SqliteProvider");

    // 2. Arrange: Create a table and insert data.
    // We use the `initialize_with_data` helper which can execute multiple statements.
    let setup_sql = "
        CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
        INSERT INTO users (id, name) VALUES (1, 'Alice');
        INSERT INTO users (id, name) VALUES (2, 'Bob');
    ";
    provider
        .initialize_with_data(setup_sql)
        .await
        .expect("Failed to initialize database with test data");

    // 3. Act: Execute a query to retrieve the data.
    let query = "SELECT id, name FROM users ORDER BY id ASC";
    let result_json = provider
        .execute_query(query)
        .await
        .expect("Failed to execute query");

    // 4. Assert: Check if the returned JSON matches the expected data.
    let expected_json = json!([
        {"id": 1, "name": "Alice"},
        {"id": 2, "name": "Bob"}
    ])
    .to_string();

    assert_eq!(result_json, expected_json);
}

/// Verifies that each in-memory provider instance is isolated from the others.
/// This is crucial for ensuring that tests do not interfere with each other.
#[tokio::test]
async fn test_sqlite_in_memory_is_isolated() {
    setup_tracing();

    // 1. Create first provider and initialize it.
    let provider1 = SqliteProvider::new(":memory:")
        .await
        .expect("Failed to create provider 1");
    provider1
        .initialize_with_data("CREATE TABLE t1 (id INTEGER); INSERT INTO t1 (id) VALUES (1);")
        .await
        .expect("Failed to initialize provider 1");

    // 2. Create a second provider. It should be a completely separate database.
    let provider2 = SqliteProvider::new(":memory:")
        .await
        .expect("Failed to create provider 2");

    // 3. Assert that the table from provider1 does not exist in provider2.
    let result = provider2.execute_query("SELECT * FROM t1").await;
    assert!(
        result.is_err(),
        "Querying table from provider1 on provider2 should fail"
    );

    let error = result.unwrap_err();
    match error {
        PromptError::StorageOperationFailed(msg) => {
            assert!(
                msg.contains("no such table: t1"),
                "Expected 'no such table' error, but got: {msg}"
            );
        }
        _ => panic!("Expected StorageOperationFailed, but got {error:?}"),
    }
}

/// Verifies that cloning a provider shares the database, and that datetime queries work.
#[tokio::test]
async fn test_sqlite_provider_shared_db_and_datetime_query() {
    setup_tracing();

    // 1. Setup: Create a new in-memory SQLite provider.
    // Cloning this provider will share the same underlying in-memory database.
    let provider1 = SqliteProvider::new(":memory:")
        .await
        .expect("Failed to create SqliteProvider");

    // 2. Arrange: Create a table and insert data with dates.
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let yesterday = (Utc::now() - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    let tomorrow = (Utc::now() + chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();

    let setup_sql = format!(
        "
        CREATE TABLE events (name TEXT, event_date TEXT);
        INSERT INTO events (name, event_date) VALUES ('Past Event', '{yesterday}');
        INSERT INTO events (name, event_date) VALUES ('Today Event 1', '{today}');
        INSERT INTO events (name, event_date) VALUES ('Today Event 2', '{today}');
        INSERT INTO events (name, event_date) VALUES ('Future Event', '{tomorrow}');
    "
    );

    provider1
        .initialize_with_data(&setup_sql)
        .await
        .expect("Failed to initialize database with test data");

    // 3. Act: Clone the provider and execute a query for today's events.
    // This demonstrates that the cloned provider shares the same database state.
    let provider2 = provider1.clone();
    let query = format!("SELECT name FROM events WHERE event_date = '{today}' ORDER BY name;");
    let result_json = provider2
        .execute_query(&query)
        .await
        .expect("Failed to execute query on cloned provider");

    // 4. Assert: Check if the returned JSON contains only today's events.
    let expected_json = json!([
        {"name": "Today Event 1"},
        {"name": "Today Event 2"}
    ])
    .to_string();

    assert_eq!(result_json, expected_json);
}
