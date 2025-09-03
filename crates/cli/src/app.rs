//! # TUI Application State
//!
//! This module defines the core state and logic for the interactive TUI application.

use crate::api_client::{ApiClient, DocumentResponse, UserListResponse};
use anyhow::Result;
use keyring::Entry;

const KEYRING_SERVICE: &str = "anyrag-cli";
const KEYRING_USERNAME: &str = "user";

/// Represents the main tabs of the TUI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Db,
    Users,
    Settings,
}

/// Represents the different input modes for the TUI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMode {
    /// The user is navigating the UI.
    Normal,
    /// The user is editing text in an input box.
    Editing,
}

/// Represents the authentication state of the application.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthState {
    /// The user is browsing as a guest.
    Guest,
    /// The user is authenticated with a specific role.
    Authenticated { role: String },
}

/// The core state for the TUI application.
pub struct App {
    /// `true` if the application is running, `false` to exit.
    pub running: bool,
    /// The current authentication state.
    pub auth_state: AuthState,
    /// The currently active tab.
    pub active_tab: Tab,
    /// A message to display in the status bar or as a popup.
    pub status: String,
    /// The keyring entry for securely storing the JWT.
    keyring_entry: Entry,
    /// The client for making API calls.
    pub api_client: ApiClient,
    /// The list of documents fetched from the server.
    pub documents: Vec<DocumentResponse>,
    /// The list of all users fetched from the server (for root).
    pub users: Vec<UserListResponse>,
    /// The current input mode.
    pub input_mode: InputMode,
    /// The text currently in the input box.
    pub input_text: String,
}

impl App {
    /// Creates a new instance of the application state.
    ///
    /// It checks the OS keychain for an existing token to determine the initial
    /// authentication state.
    pub fn new() -> Result<Self> {
        let entry = Entry::new(KEYRING_SERVICE, KEYRING_USERNAME)?;
        let auth_state = if entry.get_password().is_ok() {
            AuthState::Authenticated {
                role: "unknown".to_string(),
            }
        } else {
            AuthState::Guest
        };

        let active_tab = Tab::Db;
        let status = if matches!(auth_state, AuthState::Authenticated { .. }) {
            "Welcome back! Verifying token...".to_string()
        } else {
            "Enter URL to ingest. Press <Enter> to submit, <Esc> to quit.".to_string()
        };
        let api_client = ApiClient::new("http://localhost:9090".to_string())?;

        Ok(Self {
            running: true,
            auth_state,
            active_tab,
            status,
            keyring_entry: entry,
            api_client,
            documents: Vec::new(),
            users: Vec::new(),
            input_mode: InputMode::Editing,
            input_text: String::new(),
        })
    }

    /// Sets the `running` flag to false to exit the main loop.
    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Handles the logout process.
    ///
    /// This method deletes the token from the keychain, updates the application
    /// state to Guest, and keeps the view on the `Db` tab.
    pub fn logout(&mut self) -> Result<()> {
        self.keyring_entry.delete_credential()?;
        self.auth_state = AuthState::Guest;
        self.active_tab = Tab::Db;
        self.documents.clear();
        self.users.clear();
        self.status = "Logout successful. You are now browsing as a guest.".to_string();
        Ok(())
    }

    /// Fetches all users from the server and updates the app state.
    pub async fn fetch_users(&mut self) {
        if self.get_role() != "root" {
            self.status = "Only root users can view the user list.".to_string();
            return;
        }

        self.status = "Fetching users...".to_string();
        match self.api_client.get_users().await {
            Ok(users) => {
                self.users = users;
                self.status = format!("Successfully fetched {} users.", self.users.len());
            }
            Err(e) => {
                self.status = format!("Error fetching users: {e}");
            }
        }
    }

    /// Stores a new token in the keychain, fetches user role, and updates the state.
    pub async fn login(&mut self, token: &str) -> Result<()> {
        self.keyring_entry.set_password(token)?;
        self.status = "Login successful! Fetching user details...".to_string();

        self.verify_token().await;
        Ok(())
    }

    /// Verifies the token with the server and updates the user role.
    pub async fn verify_token(&mut self) {
        match self.api_client.get_me().await {
            Ok(user) => {
                self.auth_state = AuthState::Authenticated { role: user.role };
                self.status = format!("Welcome, {}!", self.get_role());
                self.active_tab = Tab::Db;
            }
            Err(e) => {
                // If token is invalid, log out.
                let _ = self.logout();
                self.status = format!("Authentication failed: {e}. You are now a guest.");
            }
        }
    }

    /// Fetches documents from the server and updates the app state.
    pub async fn fetch_documents(&mut self) {
        self.status = "Fetching documents...".to_string();
        match self.api_client.get_documents().await {
            Ok(docs) => {
                self.documents = docs;
                self.status = format!("Successfully fetched {} documents.", self.documents.len());
            }
            Err(e) => {
                self.status = format!("Error fetching documents: {e}");
            }
        }
    }

    /// Submits the URL from the input box for ingestion and refreshes the document list.
    pub async fn submit_ingestion(&mut self) {
        let url_to_ingest = self.input_text.clone();
        // Reset UI state first
        self.input_mode = InputMode::Normal;
        self.input_text.clear();

        if url_to_ingest.is_empty() {
            self.status =
                "Ingestion cancelled. Press 'i' to edit input or <Esc> to quit.".to_string();
            return;
        }

        self.status = format!("Sending {url_to_ingest} for ingestion...");

        match self.api_client.ingest_url(&url_to_ingest).await {
            Ok(_) => {
                self.status =
                    "Ingestion request sent successfully. Refreshing documents...".to_string();
                // Refresh the document list to show the new item.
                self.fetch_documents().await;
            }
            Err(e) => {
                self.status = format!("Error ingesting URL: {e}");
            }
        }
    }

    /// Cycles to the next tab in the sequence.
    pub fn next_tab(&mut self) {
        self.active_tab = match self.active_tab {
            Tab::Db => {
                if self.get_role() == "root" {
                    Tab::Users
                } else {
                    Tab::Settings
                }
            }
            Tab::Users => Tab::Settings,
            Tab::Settings => Tab::Db,
        };
    }

    /// Helper to get the current user role as a string slice.
    pub fn get_role(&self) -> &str {
        match &self.auth_state {
            AuthState::Authenticated { role } => role,
            AuthState::Guest => "guest",
        }
    }
}
