# Application Security, Privacy, and Ownership Plan v3

This document outlines the architectural plan for implementing a multi-tenant system with robust data privacy, authentication, authorization, advanced sharing controls, and data lifecycle management within the `anyrag` application.

## 1. Guiding Principles

-   **Privacy by Design**: Integrate privacy and security controls into the application from the ground up.
-   **Principle of Least Privilege**: Users and system components should only have access to the data and operations necessary for their function.
-   **Separation of Concerns**: System components should be modular and have a single, well-defined responsibility.
-   **Data Governance**: Implement clear rules for data retention, ownership, and deletion.

## 2. System Architecture: The `core-access` Crate

To cleanly separate security and access control logic from the main application logic, a new workspace crate named **`core-access`** will be created.

-   **Purpose**: This crate will be the single source of truth for all authentication and authorization decisions.
-   **Internal Modules**:
    -   `src/authn/`: Handles **Authentication** (AuthN) - verifying who a user is.
    -   `src/authz/`: Handles **Authorization** (AuthZ) - determining what a user can do (the "gate").
    -   `src/models/`: Defines core data structures like `User`, `Group`, etc.

## 3. Authentication & User Identity

-   **Mechanism**: A JSON Web Token (JWT) based system will be used.
-   **User Pseudonymization**: To avoid storing raw emails, a user's primary identifier (`user_id`) will be a **HMAC-SHA256** hash of their email, using a secret server-side key ("pepper").

## 4. Data Ownership & Access Control Model

This model is designed to support public content, private user-owned content, and collaborative sharing, with efficiency as a key goal.

### 4.1. Public vs. Private Content

-   **Configuration**: A new server configuration option, `ALLOW_GUEST_INGESTION` (boolean, default: `false`), will be added. If `false`, all ingest endpoints will require a valid JWT.
-   **Database Schema**: The `owner_id` column in all content tables (`raw_content`, `faq_kb`, etc.) will be **nullable**.
    -   **Private Content**: When an authenticated user ingests content, the `owner_id` is set to their `user_id`.
    -   **Public Content**: If `ALLOW_GUEST_INGESTION` is `true`, an unauthenticated request will result in `owner_id` being `NULL`.

### 4.2. Ownership in Metadata for Efficient Filtering

To enable performant, query-time authorization, ownership information must be stored directly within searchable metadata.

-   **Vector Database (SQLite `faq_kb`, `articles`)**: The `owner_id` (or a special value for public) will be stored alongside the vector. Vector search queries **must** filter on this metadata field *before* performing the similarity search.
-   **Knowledge Graph (IndraDB)**: Every entity and fact related to user-ingested content will have an `owner_id` property. Graph queries **must** filter on this property.

### 4.3. Group Policies & Sharing

A robust sharing model will enable collaboration.

-   **Database Schema**: New tables `groups`, `group_memberships`, and a polymorphic `resource_shares` table will be created to manage sharing with both individual users and groups. The `resource_shares` table will specify a `grantee_id` (user or group ID) and a `grantee_type` (`'user'` or `'group'`).

### 4.4. Complete Authorization Logic

A user is authorized to access a resource if any of the following are true:
1.  The resource's `owner_id` is `NULL` (it's public).
2.  The user is the `owner_id` of the resource.
3.  The resource is shared directly with the user.
4.  The resource is shared with a group of which the user is a member.

## 5. Data Lifecycle & Governance

### 5.1. Time-to-Live (TTL) for Content

To manage transient data, an expiration mechanism will be implemented.

-   **Database Schema**: A nullable `expires_at` (DATETIME) column will be added to all content tables.
-   **Rules**:
    -   When a **guest user** ingests content, `expires_at` will be set to `NOW() + 24 hours` by default.
    -   When an **authenticated user** ingests content, `expires_at` will be `NULL` by default, meaning it does not expire.

### 5.2. Expired Content Wipeout

-   **API Endpoint**: A new, administrator-protected endpoint, `DELETE /admin/content/expired`, will be created.
-   **Logic**: This endpoint will execute a query to find and delete all records from content tables where `expires_at` is in the past.
-   **Trigger**: This API is designed to be called by an external, scheduled job (e.g., a cron job or cloud scheduler) to perform periodic cleanup.

## 6. API Endpoints

The server API will be expanded to support these features.

-   **Auth**: `POST /auth/login`, `POST /auth/register`
-   **Groups**: `POST /groups`, `GET /groups`, `DELETE /groups/{group_id}`
-   **Group Membership**: `POST /groups/{group_id}/members`, `DELETE /groups/{group_id}/members/{user_id}`
-   **Sharing**: `POST /share`, `DELETE /share`
-   **Admin**: `DELETE /admin/content/expired` (Requires special admin role/key)

## 7. Data Privacy Plan

-   **PII Redaction**: All ingestion pipelines will use a `PiiScanner` service to find and redact common PII patterns before data is stored.
-   **Strict Tenancy**: The authorization logic must be rigorously applied to all data retrieval operations.
-   **Right to be Forgotten**: A `DELETE /users/me` endpoint will perform a cascading delete of all user-associated data.

## 8. Implementation Roadmap

-   **Phase 1: `core-access` Crate & Authentication**
-   **Phase 2: Public/Private Content & TTL**
    -   [ ] Add `ALLOW_GUEST_INGESTION` config.
    -   [ ] Add nullable `owner_id` and `expires_at` columns.
    -   [ ] Update ingest handlers to set ownership and TTL based on authentication.
    -   [ ] Update RAG search to filter on ownership.
-   **Phase 3: Groups & Sharing**
-   **Phase 4: Governance & Privacy**
    -   [ ] Implement the `DELETE /admin/content/expired` endpoint.
    -   [ ] Develop and integrate the `PiiScanner` service.
    -   [ ] Implement the `DELETE /users/me` endpoint.