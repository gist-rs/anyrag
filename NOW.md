# IMMEDIATE ACTION PLAN: Phase 2 - Core Access & Ownership (Refined)

**Objective**: To implement a robust identity, ownership, and access control system that flexibly supports both authenticated multi-user deployments and unauthenticated single-user/guest instances. This phase refines the `core-access` crate and the authentication middleware to introduce a "Guest User" concept, ensuring all data is clearly owned without requiring a login for local or offline use.

This is the **current, focused task** for the development team.

## 1. The Core Problem & Solution

-   **Problem**: The application needs a clear ownership model, but forcing JWT authentication for all use cases (like a local CLI or single-user server) is impractical. The previous concept of `NULL` for a public `owner_id` is also ambiguous.
-   **Solution**: We will implement a refined authentication middleware. Requests with a valid JWT will be associated with the authenticated user. Requests **without** a JWT will be automatically associated with a special, deterministic **"Guest User"**. This ensures every piece of data has a non-null `owner_id`, simplifying logic and data governance while providing maximum flexibility.

## 2. Key Architectural Components

-   **`core-access` Crate**: Remains the single source of truth for user models and persistence logic. It will be used to retrieve both authenticated users and the guest user.
-   **User Resolution Middleware**: The Axum middleware will be updated to implement the following logic:
    1.  **Valid Token Present**: Decode the JWT, extract the user's unique identifier (e.g., `sub` claim), and fetch or create the user via `get_or_create_user`.
    2.  **No Token Present**: Automatically fetch or create the deterministic "Guest User" using a constant identifier (e.g., `::guest::`).
    3.  **Invalid/Expired Token Present**: Reject the request with a `401 Unauthorized` error. This is a critical security boundary.
-   **Guest User Persistence**: The `get_or_create_user` function in `core-access` will handle creating the guest user in the `users` table the first time it's requested.

## 3. Implementation Roadmap

1.  **[Workspace] Create the `core-access` Crate**:
    -   Generate a new library crate at `crates/core-access`.
    -   Add this new crate to the main workspace `Cargo.toml`.
    -   Add `anyrag-server`'s necessary dependencies for JWT handling (e.g., `jsonwebtoken`, `axum-extra`).

2.  **[Core Access] Implement User Model & Persistence**:
    -   In the `core-access` crate, define the core `User` struct (e.g., `id`, `created_at`).
    -   Implement a `get_or_create_user` function that takes an identifier and a database connection, and returns a `User`.

3.  **[Server] Implement Refined Authentication Middleware**:
    -   In `anyrag-server`, modify the authentication middleware.
    -   Update the Axum `from_request_parts` implementation to handle the three cases: valid token, no token, and invalid token.
    -   Ensure a `User` object (either authenticated or guest) is always attached to the request extensions for downstream handlers.

4.  **[Server] Apply User Middleware & Enforce Ownership**:
    -   Apply the user resolution middleware to all data modification endpoints (`/knowledge/ingest`, `/ingest/file`, etc.).
    -   Update the handlers for these endpoints to retrieve the user's ID (whether real or guest) from the request extensions.
    -   Pass this `owner_id` down into the library functions (`run_ingestion_pipeline`, etc.) to correctly populate the `owner_id` column in all relevant tables.

5.  **[Library] Implement Ownership-Based Filtering**:
    -   Modify the `hybrid_search` function to accept a required `owner_id: &str`.
    -   Update the `metadata_search` function in the `SqliteProvider`. The `WHERE` clause must be updated to select documents where `owner_id` matches the current user's ID **OR** `owner_id` matches the **Guest User's ID**. This allows all users to see public/guest content.

6.  **[Testing] Update Integration Tests**:
    -   Update the E2E tests for ingestion and search endpoints.
    -   Add a new test case to verify that unauthenticated requests are correctly processed as the "Guest User" and that data is stored with the guest's `owner_id`.
    -   Update search tests to verify that an authenticated user can see both their own content and guest content.
    -   Ensure a test exists to confirm that requests with an *invalid* token are rejected with a `401`.

## 5. Success Criteria

-   Endpoints return a `401 Unauthorized` error **only if** an invalid or expired JWT is provided.
-   Requests without a JWT are successfully processed as the "Guest User".
-   When a user (authenticated or guest) ingests new content, their unique ID is correctly stored in the `owner_id` field of all related database tables.
-   The `/search/knowledge` endpoint returns documents that are owned by the authenticated user **and** documents owned by the "Guest User".
-   All new and existing tests pass.