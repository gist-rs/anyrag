### **NOW: Implement a TUI-First CLI with Seamless Authentication**

This document outlines the immediate plan to create a secure, interactive, and user-friendly `Ratatui`-based terminal application. The primary interface will be the TUI, from which all actions, including the initial login, are performed.

#### **1. Objective**

To build a terminal application where the user is immediately presented with a `Ratatui` interface. If not authenticated, the TUI will guide them through a seamless, browser-based login process without any manual code entry. The goal is to create an integrated experience, not a set of separate command-line tools.

#### **2. Technology Choice: OAuth 2.0 Authorization Code Grant**

We will implement the **OAuth 2.0 Authorization Code Grant** using a temporary, local web server. This remains the most secure and seamless method for a desktop/CLI application to authenticate a user.

**The User Flow will be:**
1.  The user runs `anyrag-cli`.
2.  The `Ratatui` application launches.
3.  The app checks for a saved API token. If not found, it displays a welcome screen with an instruction like "Press 'L' to log in."
4.  The user presses 'L'.
5.  The app automatically opens the user's default web browser to a Google sign-in and consent page.
6.  The user signs in and clicks "Allow".
7.  Google redirects the browser back to a temporary local web server, which securely captures the authentication token.
8.  The TUI receives the token, saves it, and automatically transitions to the main dashboard view, confirming a successful login.

#### **3. Implementation Plan**

This plan is broken into two parallel efforts: enhancing the server to support the OAuth 2.0 flow and creating the new `Ratatui`-based CLI application.

---

##### **Phase 1: `anyrag-server` Enhancements**

The server must be updated to act as the OAuth 2.0 client that communicates with Google. *(This part of the plan remains unchanged, as the server's role is independent of the CLI's UI.)*

**Step 1. Add Dependencies & Configuration**
1.  Add the `openidconnect` crate to `anyrag-server/Cargo.toml`.
2.  Update `.env.example` and the configuration system to accept new, required variables for Google OAuth:
    *   `GOOGLE_OAUTH_CLIENT_ID`
    *   `GOOGLE_OAUTH_CLIENT_SECRET`
    *   `SERVER_BASE_URL` (e.g., `http://localhost:9090`) to correctly construct the `redirect_uri`.

**Step 2. Implement New Authentication Endpoints**
In `anyrag-server/src/handlers`, we will add a new module for these public-facing authentication routes:

1.  **`GET /auth/login/google`**: Redirects the user's browser to the Google OAuth consent screen. It will pass along state and PKCE challenges.
2.  **`GET /auth/callback/google`**: The `redirect_uri` that Google calls back to. It will exchange the `authorization_code` from Google for an `id_token`, create a user in our system, generate our application JWT, and finally redirect the browser back to the CLI's local server with the JWT.

---

##### **Phase 2: New `anyrag-cli` Crate (Ratatui First)**

We will create a new, dedicated crate for the command-line interface with `Ratatui` as its core.

**Step 1. Create the Crate and Add TUI Dependencies**
1.  Create a new binary crate: `cargo new anyrag/crates/cli`.
2.  Add it to the main workspace `Cargo.toml`.
3.  Add dependencies to `anyrag/crates/cli/Cargo.toml`:
    *   `ratatui` for the TUI framework.
    *   `crossterm` for terminal manipulation.
    *   `tokio` (full features) for the async runtime.
    *   `hyper` or a similar library for the temporary local web server.
    *   `reqwest` for API calls to the `anyrag-server`.
    *   `serde` and `serde_json`.
    *   `keyring` to securely store the received JWT.
    *   `open` to automatically open the browser.

**Step 2. Implement the TUI Application Shell and Login Flow**
1.  Set up the basic `Ratatui` application structure: a main loop, event handling (user input), a state management struct (`App`), and a rendering function (`ui`).
2.  On startup, the `App` state will immediately check the `keyring` for an existing token.
3.  **If no token exists**:
    *   The `ui` function will render a simple view instructing the user to press 'L' to log in.
    *   The event handler will listen for the 'L' key.
4.  **When 'L' is pressed**:
    *   The TUI will trigger the `auth` module.
    *   The `auth` module will perform the local web server OAuth flow as previously planned (start server, open browser, wait for callback).
    *   During this process, the TUI will display a "Waiting for login in browser..." message.
5.  **Upon successful login**:
    *   The received JWT is stored in the keychain.
    *   The `App` state is updated to reflect the authenticated status.
    *   The `ui` function will now render the main dashboard (e.g., the knowledge base view).

**Step 3. Implement Token Usage**
1.  A shared API client within the CLI will automatically load the JWT from the keychain and add it as a `Bearer` token to all authenticated API calls.