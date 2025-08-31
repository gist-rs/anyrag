# IMMEDIATE ACTION PLAN: Phase 2 - Core Access & Ownership

**Objective**: To implement a robust identity, ownership, and access control system. This phase introduces a new `core-access` crate, secures endpoints with JWT authentication, and ensures all data is correctly associated with an owner. This will transform the application from a single-user tool into the foundation for a multi-tenant platform.

This is the **current, focused task** for the development team, succeeding the completed database refactor.

## 1. The Core Problem & Solution

-   **Problem**: The application currently has no concept of users or data ownership. All ingested data is effectively public, and there is no security on the ingestion endpoints.
-   **Solution**: We will create a dedicated `core-access` crate to handle all user-related logic. We will implement JWT-based authentication as an Axum middleware to protect sensitive endpoints. All data creation and retrieval operations will be updated to enforce ownership rules.

## 2. Key Architectural Components

-   **`core-access` Crate**: A new library crate within the workspace (`crates/core-access`) that will be the single source of truth for user models and authentication logic.
-   **JWT Middleware**: An Axum middleware that will validate `Authorization: Bearer <token>` headers, decode the JWT to extract user claims, and make the authenticated user's ID available to downstream handlers.
-   **User Persistence**: Logic within the `core-access` crate to find an existing user or create a new one in the `users` table based on the unique identifier (e.g., the `sub` claim) from the JWT.

## 3. Implementation Roadmap

1.  **[Workspace] Create the `core-access` Crate**:
    -   Generate a new library crate at `crates/core-access`.
    -   Add this new crate to the main workspace `Cargo.toml`.
    -   Add `anyrag-server`'s necessary dependencies for JWT handling (e.g., `jsonwebtoken`, `axum-extra`).

2.  **[Core Access] Implement User Model & Persistence**:
    -   In the `core-access` crate, define the core `User` struct (e.g., `id`, `created_at`).
    -   Implement a `get_or_create_user_by_sub` function that takes a JWT subject claim and a database connection, and returns a `User`. This function will perform an `INSERT ... ON CONFLICT DO NOTHING` followed by a `SELECT`.

3.  **[Server] Implement JWT Authentication Middleware**:
    -   In `anyrag-server`, create a new module for authentication middleware.
    -   Define a `Claims` struct for the JWT payload.
    -   Implement the Axum `from_request` extractor or middleware function that:
        -   Extracts the token from the `Authorization` header.
        -   Decodes and validates the token using a secret from the environment.
        -   Uses the `get_or_create_user_by_sub` function to retrieve the user.
        -   Attaches the user ID to the request extensions for handlers to use.

4.  **[Server] Secure Endpoints & Enforce Ownership**:
    -   Apply the new JWT middleware to all data modification endpoints (e.g., `/knowledge/ingest`, `/ingest/file`, `/ingest/text`).
    -   Update the handlers for these endpoints to retrieve the authenticated user's ID from the request extensions.
    -   Pass this `owner_id` down into the library functions (`run_ingestion_pipeline`, etc.) to correctly populate the `owner_id` column in the `documents`, `faq_items`, and `content_metadata` tables.

5.  **[Library] Implement Ownership-Based Filtering**:
    -   Modify the `hybrid_search` function in `anyrag` to accept an `owner_id: Option<&str>`.
    -   Update the `metadata_search` function to use this `owner_id` in its SQL query. The `WHERE` clause should be updated to select documents where `owner_id` matches the user's ID OR `owner_id` is NULL (for public content).

6.  **[Testing] Update Integration Tests**:
    -   Write new unit tests for the functions in the `core-access` crate.
    -   Update the E2E tests for ingestion and search endpoints to include a valid authentication header in their requests.
    -   Add assertions to verify that `owner_id` is set correctly and that search results are properly filtered.

## 5. Success Criteria

-   All ingestion endpoints return a `401 Unauthorized` error if a valid JWT is not provided.
-   When a user ingests new content, their unique ID is correctly stored in the `owner_id` field of all related database tables.
-   The `/search/knowledge` endpoint only returns documents that are owned by the authenticated user or are public (owner_id is NULL).
-   All new and existing tests pass.