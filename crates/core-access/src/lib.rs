//! # Core Access Crate
//!
//! This crate is the central authority for all identity, authentication (AuthN),
//! and authorization (AuthZ) logic for the `anyrag` application.

pub const GUEST_USER_IDENTIFIER: &str = "::guest::";

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;
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
///
/// # Security
///
/// The `role_override` parameter is a potential security risk and should only
/// be used in trusted, internal contexts like tests or administrative scripts.
/// It MUST NOT be exposed to external API calls, as this could allow a user
/// to escalate their privileges.
pub async fn get_or_create_user(
    db: &Database,
    user_identifier: &str,
    role_override: Option<&str>,
) -> Result<User, CoreAccessError> {
    info!("[core_access] get_or_create_user for identifier: '{user_identifier}'");
    let conn = db.connect()?;
    let user_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, user_identifier.as_bytes()).to_string();
    info!("[core_access] Calculated user_id: '{user_id}'");

    // 1. Try to SELECT the user first for maximum compatibility.
    let mut rows = conn
        .query(
            "SELECT id, role, created_at FROM users WHERE id = ?",
            params![user_id.clone()],
        )
        .await?;

    if let Some(row) = rows.next().await? {
        // User exists, parse it.
        let mut user = User::try_from(&row)?;
        info!("[core_access] Found existing user: {user:?}");

        // If a role override is provided and it's different, update the existing user's role.
        if let Some(new_role) = role_override {
            if user.role != new_role {
                info!(
                    "[core_access] Updating user {} role from '{}' to '{}'",
                    user.id, user.role, new_role
                );
                conn.execute(
                    "UPDATE users SET role = ? WHERE id = ?",
                    params![new_role, user.id.clone()],
                )
                .await?;
                user.role = new_role.to_string(); // Update the struct to be returned
            }
        }

        return Ok(user);
    }

    info!("[core_access] User not found, creating new user.");
    // 2. User does not exist. Determine role.
    let role = role_override.unwrap_or("user");
    info!("[core_access] Determined role for new user: '{role}'");

    // 2.5. If user doesn't exist, INSERT. If the INSERT fails due to a UNIQUE
    // constraint violation, it means a concurrent process created the user
    // between our SELECT and INSERT (a race condition). In this case, we can
    // safely ignore the error and proceed to the final SELECT.
    if let Err(e) = conn
        .execute(
            "INSERT INTO users (id, role) VALUES (?, ?)",
            params![user_id.clone(), role],
        )
        .await
    {
        if !e.to_string().contains("UNIQUE constraint failed") {
            return Err(e.into());
        }
    }

    // 3. SELECT the user, which is now guaranteed to exist.
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

    let user = User::try_from(&row)?;
    info!("[core_access] Returning newly created user: {:?}", user);
    Ok(user)
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

        // 2. Act: First call should create the user, explicitly making them root.
        let user1 = get_or_create_user(&db, user_identifier, Some("root"))
            .await
            .unwrap();

        // 3. Assert: Check the created user
        let expected_id =
            Uuid::new_v5(&Uuid::NAMESPACE_URL, user_identifier.as_bytes()).to_string();
        assert_eq!(user1.id, expected_id);
        assert_eq!(user1.role, "root", "The first user should be root");

        // 4. Act: Second call should retrieve the same user
        let user2 = get_or_create_user(&db, user_identifier, None)
            .await
            .unwrap();

        // 5. Assert: Check that the retrieved user is identical
        assert_eq!(user1.id, user2.id);
        assert_eq!(user1.role, user2.role);
        assert_eq!(user1.created_at.timestamp(), user2.created_at.timestamp());

        // 6. Act: Create a second user
        let second_user_identifier = "another@example.com";
        let user3 = get_or_create_user(&db, second_user_identifier, None)
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
        let guest_user = get_or_create_user(&db, GUEST_USER_IDENTIFIER, None)
            .await
            .unwrap();

        // 3. Assert: The guest user should NOT be root, even if they are the first user.
        assert_eq!(
            guest_user.role, "user",
            "The guest user should never be root"
        );

        // 4. Act: Create a normal user, explicitly making them root.
        let normal_user = get_or_create_user(&db, "first.real.user@example.com", Some("root"))
            .await
            .unwrap();

        // 5. Assert: This user should now be root because they are the first non-guest user.
        assert_eq!(
            normal_user.role, "root",
            "The first non-guest user should be root"
        );

        // 6. Act: Create a second normal user.
        let second_normal_user = get_or_create_user(&db, "second.real.user@example.com", None)
            .await
            .unwrap();

        // 7. Assert: The second user should have the 'user' role.
        assert_eq!(
            second_normal_user.role, "user",
            "The second non-guest user should have the 'user' role"
        );
    }
}
