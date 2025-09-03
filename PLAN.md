### **MASTER PLAN: `anyrag-cli` - An Interactive TUI Dashboard**

This document outlines the master plan for creating a new, feature-rich command-line interface (CLI) for the `anyrag` application, featuring an interactive terminal user interface (TUI) built with `Ratatui`.

#### **1. High-Level Objective**

To provide a powerful, terminal-based experience for developers and administrators to manage and interact with the `anyrag` service. The application's primary interface will be a full-screen `Ratatui` TUI, from which users can manage the knowledge base, users, and perform searches interactively.

#### **2. Core Technologies**

*   **TUI Framework**: `Ratatui` for building the interactive dashboard.
*   **Terminal Backend**: `crossterm` for terminal manipulation and event handling.
*   **Async Runtime**: `tokio` to handle asynchronous operations, especially API calls.
*   **Secure Storage**: `keyring` to store authentication tokens securely in the native OS keychain.
*   **HTTP Client**: `reqwest` for all communication with the `anyrag-server` API.
*   **Browser Interaction**: `open` to automatically open URLs for the authentication flow.

#### **3. Proposed Architecture**

A new crate, `anyrag-cli`, will be created within the workspace. Its architecture will be centered around the `Ratatui` event loop model:

*   **`main.rs`**: Entry point, responsible for initializing the terminal, creating the `App` state, and running the main TUI loop.
*   **`auth.rs`**: Handles the entire browser-based authentication flow, triggered from within the TUI.
*   **`api_client.rs`**: A centralized client for all `anyrag-server` API interactions. It will automatically load the JWT from the keychain and attach it to requests.
*   **`tui/`**: A dedicated module for the `Ratatui` dashboard.
    *   **`app.rs`**: Manages the TUI's state (e.g., authentication status, current tab, lists of documents, API data, popups).
    *   **`ui.rs`**: Contains the rendering logic for all widgets and layouts.
    *   **`event.rs`**: Handles user input (key presses) and application events (like API responses).
    *   **`components/`**: A sub-module for reusable TUI components (e.g., tables, popups, input boxes).

#### **4. Feature Roadmap**

##### **Phase 1: TUI Foundation & Authentication**

This phase focuses on creating a functional TUI application that can authenticate with the server.

1.  **TUI Application Shell**: Implement the main event loop, terminal setup/teardown, and the basic `App` state machine.
2.  **Authentication Flow**:
    *   On startup, the TUI checks for a token.
    *   If no token exists, it displays a "Login" view.
    *   A key press (`L`) triggers the seamless, browser-based OAuth 2.0 flow.
    *   The TUI displays a "Waiting..." message while the user authenticates in their browser.
    *   Upon success, the token is stored in the keychain, and the TUI state transitions to "Authenticated".
3.  **Initial View: Knowledge Base Tab**:
    *   After authentication, the TUI displays the main dashboard.
    *   The first tab will be a read-only, scrollable list of documents from the knowledge base, fetched from a new `/documents` endpoint on the server.
    *   Columns: Title, Source URL, Owner ID.

##### **Phase 2: Interactive CRUD & User Management**

This phase builds on the foundation by adding interactive management features to the TUI.

1.  **Knowledge Base CRUD**:
    *   **Detailed View (`<Enter>`)**: Show the full content, FAQs, and metadata for a selected document.
    *   **Deletion (`d`)**: Delete the selected document, showing a confirmation popup before sending the API request.
    *   **Re-ingest (`r`)**: Re-run the ingestion pipeline for the selected document's source URL.
    *   **New Ingestion (`n`)**: Open an input popup to paste a new URL for ingestion.
2.  **Users Tab**:
    *   Add a second tab to the TUI.
    *   Display a scrollable list of all users in the system.
    *   **Filtering**: Pressing `<Enter>` on a user filters the Knowledge Base tab to show only documents owned by that user.

##### **Phase 3: Advanced TUI Features**

This phase adds more sophisticated and user-friendly features to the dashboard.

1.  **Interactive Search**: A dedicated "Search" tab with an input box. As the user types, they can trigger a RAG search and see the results displayed within the TUI.
2.  **Status & Help Bar**: A persistent footer showing the current user, server connection status, and context-sensitive keybinding hints.
3.  **File Ingestion**: Add a feature (e.g., keybinding `u` for upload) that opens an input box for a local file path to ingest files like PDFs.

#### **5. Future Enhancements**

*   A dashboard "Home" tab with summary statistics (total documents, users, etc.).
*   Support for managing knowledge graph facts.
*   A real-time log viewer tab.
*   Editing content directly within the TUI.