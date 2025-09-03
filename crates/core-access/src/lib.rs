//! # Core Access Crate
//!
//! This crate is the central authority for all identity, authentication (AuthN),
//! and authorization (AuthZ) logic for the `anyrag` application.

pub const GUEST_USER_IDENTIFIER: &str = "::guest::";

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use turso::{Database, Error as TursoError, Row, params};
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum CoreAccessError {
    #[error("Database error: {0}")]
    Database(#[from] TursoError),
    #[error("Failed to create or find user for identifier: {0}")]
    UserPersistenceFailed(String),
    #[error("Data integrity error: {0}")]
    DataIntegrity(String),
}

/// Represents a user in the system.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    /// The unique, deterministic ID of the user (UUIDv5 from an external identifier).
    pub id: String,
    /// The user's role (e.g., 'user', 'root').
    pub role: String,
    /// The timestamp when the user was first created.
    pub created_at: DateTime<Utc>,
}

impl TryFrom<&Row> for User {
    type Error = CoreAccessError;

    fn try_from(row: &Row) -> std::result::Result<Self, Self::Error> {
        let created_at_str: String = row.get(2)?;
        let created_at =
            chrono::NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
                .map_err(|e| {
                    CoreAccessError::DataIntegrity(format!(
                        "Failed to parse date '{created_at_str}': {e}"
                    ))
                })?;

        Ok(User {
            id: row.get(0)?,
            role: row.get(1)?,
            created_at,
        })
    }
}

/// Finds a user by their unique identifier (e.g., email or token sub),
/// creating them if they don't exist.
///
/// This function creates a deterministic UUIDv5 from the identifier to use as
/// the primary key, ensuring idempotency.
pub async fn get_or_create_user(
    db: &Database,
    user_identifier: &str,
) -> Result<User, CoreAccessError> {
    let conn = db.connect()?;
    let user_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, user_identifier.as_bytes()).to_string();

    // 1. Try to SELECT the user first for maximum compatibility.
    let mut rows = conn
        .query(
            "SELECT id, role, created_at FROM users WHERE id = ?",
            params![user_id.clone()],
        )
        .await?;

    if let Some(row) = rows.next().await? {
        // User exists, parse and return it.
        return User::try_from(&row);
    }

    // 2. User does not exist. Determine role.
    // A guest user can never be root. The first non-guest user becomes root.
    let role = if user_identifier == GUEST_USER_IDENTIFIER {
        "user"
    } else {
        let root_exists = conn
            .query("SELECT 1 FROM users WHERE role = 'root' LIMIT 1", ())
            .await?
            .next()
            .await?
            .is_some();

        if !root_exists { "root" } else { "user" }
    };

    // Insert the new user with the determined role.
    conn.execute(
        "INSERT INTO users (id, role) VALUES (?, ?)",
        params![user_id.clone(), role],
    )
    .await?;

    // 3. SELECT the newly created user to get all fields (like created_at).
    let mut rows = conn
        .query(
            "SELECT id, role, created_at FROM users WHERE id = ?",
            params![user_id],
        )
        .await?;

    let row = rows
        .next()
        .await?
        .ok_or_else(|| CoreAccessError::UserPersistenceFailed(user_identifier.to_string()))?;

    User::try_from(&row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyrag::providers::db::sqlite::SqliteProvider;

    #[tokio::test]
    async fn test_get_or_create_user_flow() {
        // 1. Arrange
        let provider = SqliteProvider::new(":memory:").await.unwrap();
        provider.initialize_schema().await.unwrap();
        let db = provider.db;
        let user_identifier = "test@example.com";

        // 2. Act: First call should create the user
        let user1 = get_or_create_user(&db, user_identifier).await.unwrap();

        // 3. Assert: Check the created user
        let expected_id =
            Uuid::new_v5(&Uuid::NAMESPACE_URL, user_identifier.as_bytes()).to_string();
        assert_eq!(user1.id, expected_id);
        assert_eq!(user1.role, "root", "The first user should be root");

        // 4. Act: Second call should retrieve the same user
        let user2 = get_or_create_user(&db, user_identifier).await.unwrap();

        // 5. Assert: Check that the retrieved user is identical
        assert_eq!(user1.id, user2.id);
        assert_eq!(user1.role, user2.role);
        assert_eq!(user1.created_at.timestamp(), user2.created_at.timestamp());

        // 6. Act: Create a second user
        let second_user_identifier = "another@example.com";
        let user3 = get_or_create_user(&db, second_user_identifier)
            .await
            .unwrap();

        // 7. Assert: The second user should have the 'user' role
        assert_ne!(user1.id, user3.id);
        assert_eq!(
            user3.role, "user",
            "The second user should have the 'user' role"
        );
    }

    #[tokio::test]
    async fn test_guest_user_is_never_root() {
        // 1. Arrange
        let provider = SqliteProvider::new(":memory:").await.unwrap();
        provider.initialize_schema().await.unwrap();
        let db = provider.db;

        // 2. Act: Create guest user first.
        let guest_user = get_or_create_user(&db, GUEST_USER_IDENTIFIER)
            .await
            .unwrap();

        // 3. Assert: The guest user should NOT be root, even if they are the first user.
        assert_eq!(
            guest_user.role, "user",
            "The guest user should never be root"
        );

        // 4. Act: Create a normal user.
        let normal_user = get_or_create_user(&db, "first.real.user@example.com")
            .await
            .unwrap();

        // 5. Assert: This user should now be root because they are the first non-guest user.
        assert_eq!(
            normal_user.role, "root",
            "The first non-guest user should be root"
        );

        // 6. Act: Create a second normal user.
        let second_normal_user = get_or_create_user(&db, "second.real.user@example.com")
            .await
            .unwrap();

        // 7. Assert: The second user should have the 'user' role.
        assert_eq!(
            second_normal_user.role, "user",
            "The second non-guest user should have the 'user' role"
        );
    }
}
