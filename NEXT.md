### **NEXT: Implement Root User Role**

This document outlines the plan for introducing a "root" or "administrator" user role into the `anyrag` system.

#### **1. Objective**

To create a special user role with elevated privileges for system-wide administration. This is a critical feature for managing a multi-user deployment, allowing an administrator to view and manage all content, regardless of ownership.

#### **2. The "First User is Root" Principle**

To simplify the initial setup and avoid complex user invitation flows, we will adopt the "first user is root" principle.

*   **Mechanism**: The very first user to be created in the database via a successful OAuth 2.0 authentication will be automatically designated as the root user.
*   **Implementation**: When `core_access::get_or_create_user` is called and a new user record is about to be inserted, the logic will first check if any other users with the "root" role already exist. If none exist, the new user will be granted this role.

#### **3. Implementation Plan**

This feature requires changes in the `core-access` crate and will be leveraged by the `anyrag-server` and `anyrag-cli`.

**Step 1: Database Schema Update**
1.  Modify the `users` table schema in `anyrag/crates/lib/src/providers/db/sqlite/sql.rs`.
2.  Add a new column, such as `role TEXT NOT NULL DEFAULT 'user'`. This column can store roles like 'user' and 'root'. A boolean `is_root` column is also an option.

**Step 2: Update `core-access` Logic**
1.  Modify the `get_or_create_user` function in `anyrag/crates/core-access/src/lib.rs`.
2.  When a new user is being created (i.e., the initial `SELECT` finds no user), perform a check: `SELECT 1 FROM users WHERE role = 'root' LIMIT 1`.
3.  If this check returns no rows, the subsequent `INSERT` statement for the new user must set their `role` to `'root'`.
4.  The `User` struct will be updated to include the `role` field, so it is available to the rest of the application.

**Step 3: Update Server Authorization Logic (Future)**
*   Endpoints that require root privileges (e.g., an endpoint to list all users) will need to be protected. An Axum middleware or extractor can be created to check if `AuthenticatedUser.0.role == "root"`.

**Step 4: Update CLI TUI**
*   The `anyrag-cli` TUI will use the user's role to conditionally render certain UI elements.
*   The **Users Tab** will only be visible and accessible if the authenticated user's `role` is "root".
*   The **DB Tab** will show all documents to a root user, but only owned documents + guest documents to a regular user. This will be handled by passing the user's role and ID to the server's API endpoints.