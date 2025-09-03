### **MASTER PLAN: An Interactive TUI Dashboard for `anyrag`**

This document outlines the master plan for the `anyrag-cli`, a TUI-first, interactive dashboard for managing and interacting with the `anyrag` service, built with `Ratatui`.

#### **1. High-Level Objective**

To create a seamless, powerful, and user-friendly terminal application. The primary interface will be a full-screen, tab-based dashboard that guides the user from authentication through to data management and search, with special privileges for an administrative "root" user.

#### **2. Core Technologies**

*   **TUI Framework**: `Ratatui` for building the interactive dashboard.
*   **Terminal Backend**: `crossterm` for terminal manipulation and event handling.
*   **Async Runtime**: `tokio` for handling asynchronous operations like API calls and user input.
*   **Secure Storage**: `keyring` to store authentication tokens securely in the native OS keychain.
*   **HTTP Client**: `reqwest` for all communication with the `anyrag-server` API.
*   **Browser Interaction**: `open` to automatically open URLs for the authentication flow.

#### **3. Proposed TUI Architecture & User Flow**

The application will be built around a stateful, tab-based interface.

1.  **Initial View (`Auth` Tab)**: On startup, the user is presented with the `Auth` tab, offering two choices:
    *   **Login with Google**: Initiates the secure, browser-based OAuth 2.0 flow. The first user to do this becomes the root user.
    *   **Continue as Guest**: Allows the user to proceed with limited, public-only access.

2.  **Main View (Post-Authentication)**: After logging in or choosing guest mode, the user is taken to the main dashboard, with the `DB` tab active.

3.  **Tabbed Navigation**: The user can navigate between the following tabs:
    *   **`DB` (Documents)**: View and manage knowledge base documents. The view is filtered based on user role (user sees their own data, root sees all data).
    *   **`Users`**: (Root only) View a list of all users in the system.
    *   **`Settings`**: Manage CLI settings, including the option to log out.
    *   **`Auth`**: The initial login screen, which is returned to after logging out.

#### **4. Feature Roadmap**

##### **Phase 1: TUI Foundation & Authentication**

This phase focuses on creating a functional TUI application that can authenticate and display the basic tab structure.

1.  **TUI Application Shell**: Implement the main `Ratatui` event loop, terminal setup/teardown, and the core `App` state machine that manages the active tab and authentication status.
2.  **Implement the `Auth` Tab**:
    *   This will be the default view if no token is found in the keychain.
    *   It will display a selectable list with "Login with Google" and "Continue as Guest".
3.  **Implement the Login Flow**:
    *   Selecting "Login with Google" will trigger the seamless, browser-based OAuth 2.0 flow (as defined in `NOW.md`).
    *   The TUI will display a "Waiting for login in browser..." status message.
    *   On success, the JWT is stored in the keychain, the user's role is fetched, and the state transitions to authenticated.
4.  **Implement the `Settings` Tab**:
    *   Create a simple settings view.
    *   Implement the "Logout" action, which will clear the token from the keychain and switch the TUI back to the `Auth` tab.

##### **Phase 2: The DB (Documents) Tab**

This phase implements the core functionality for viewing knowledge base content.

1.  **Server Endpoint**: Create a `GET /documents` endpoint on the server that respects ownership (returns owned + guest documents for a user, or just guest documents for a guest).
2.  **TUI View**:
    *   After authentication, the TUI will automatically switch to the `DB` tab.
    *   It will call the `/documents` endpoint and display the results in a scrollable, sortable table.
    *   Columns: Title, Source URL, Owner ID, Created At.
3.  **Detailed View**: Pressing `<Enter>` on a document will navigate to a new screen showing the full document content, its extracted FAQs, and associated metadata.

##### **Phase 3: Root User Privileges & The Users Tab**

This phase introduces the administrator role and its associated features.

1.  **Implement Root User**: Fulfill the plan in `NEXT.md` on the server-side to designate the first authenticated user as "root".
2.  **Enhance Server Endpoints**:
    *   Modify `GET /documents` to return *all* documents if the requesting user has the "root" role.
    *   Create a new, root-protected `GET /users` endpoint that returns a list of all users.
3.  **Update TUI**:
    *   The TUI will fetch and store the user's role upon login.
    *   The **`Users` Tab** will be conditionally rendered and only accessible if the user's role is "root". It will display the list of all users from the new endpoint.
    *   The **`DB` Tab** will display the appropriate (filtered or complete) list of documents based on the user's role.

##### **Phase 4: Advanced Interactivity & Management**

This phase adds full management capabilities to the TUI.

1.  **Interactive CRUD in `DB` Tab**:
    *   **New Ingestion (`n`)**: Open an input popup to paste a new URL for ingestion.
    *   **Re-ingest (`r`)**: Re-run the ingestion pipeline for the selected document's URL.
    *   **Deletion (`d`)**: Delete the selected document, showing a confirmation popup first.
2.  **Interactive Search**: Add a dedicated search input to the `DB` tab (or a new `Search` tab) to perform RAG queries and display results directly within the TUI.
3.  **Status & Help Bar**: A persistent footer showing the current user, role, and context-sensitive keybinding hints.