//! # anyrag-cli: An Interactive TUI for `anyrag`
//!
//! This is the main entry point for the `anyrag` terminal user interface.

mod api_client;
mod app;
mod auth;
mod ui;

use crate::{
    app::{App, AuthState, InputMode, Tab},
    ui::ui,
};
use anyhow::Result;
use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyCode, KeyEvent, KeyEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, prelude::*};
use std::fs::File;
use std::io::{self, Stdout};
use std::time::Duration;
use tracing_subscriber::{fmt, EnvFilter};

// --- Main Application Entry ---

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging to a file to prevent interfering with the TUI.
    let log_file = File::create("anyrag-cli.log")?;
    let subscriber = fmt::Subscriber::builder()
        .with_writer(log_file) // Direct logs to the file
        .with_env_filter(EnvFilter::from_default_env()) // Allow RUST_LOG override, e.g. RUST_LOG=info
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Setup terminal
    let mut terminal = setup_terminal()?;

    // Create app instance
    let mut app = App::new()?;

    // On startup, if the user has a token, verify it.
    // Then, fetch documents for the initial view (works for both Guest and Authenticated).
    if matches!(app.auth_state, AuthState::Authenticated { .. }) {
        app.verify_token().await;
    }
    app.fetch_documents().await;

    // Run the main TUI loop
    run_app(&mut terminal, &mut app).await?;

    // Restore terminal state
    restore_terminal(&mut terminal)?;

    Ok(())
}

// --- TUI Rendering and Event Loop ---

type TerminalBackend = Terminal<CrosstermBackend<Stdout>>;

/// The main application loop.
async fn run_app(terminal: &mut TerminalBackend, app: &mut App) -> Result<()> {
    while app.running {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                // Handle key presses based on the current input mode.
                Event::Key(key) if key.kind == KeyEventKind::Press => match app.input_mode {
                    InputMode::Normal => handle_normal_keys(key, app, terminal).await?,
                    InputMode::Editing => handle_editing_keys(key, app).await?,
                },
                // Handle paste events only when in editing mode.
                Event::Paste(text) => {
                    if app.input_mode == InputMode::Editing {
                        app.input_text.push_str(&text);
                    }
                }
                // Ignore other events like mouse clicks, resizes, etc.
                _ => {}
            }
        }
    }
    Ok(())
}

/// Handles key presses when in `InputMode::Normal`.
async fn handle_normal_keys(
    key: KeyEvent,
    app: &mut App,
    terminal: &mut TerminalBackend,
) -> Result<()> {
    // --- Global Keybindings ---
    match key.code {
        // 'i' enters editing mode to use the input box.
        KeyCode::Char('i') => {
            app.input_mode = InputMode::Editing;
            app.status = "Editing mode: Press <Enter> to submit, <Esc> to cancel.".to_string();
            return Ok(());
        }
        KeyCode::Esc => {
            app.quit();
            return Ok(());
        }
        KeyCode::Tab => {
            app.next_tab();
            // Fetch data if switching to a data-heavy tab for the first time
            if matches!(app.active_tab, Tab::Db) && app.documents.is_empty() {
                app.fetch_documents().await;
            } else if matches!(app.active_tab, Tab::Users) && app.users.is_empty() {
                app.fetch_users().await;
            }
            return Ok(());
        }
        _ => {}
    }

    // --- Tab-specific Keybindings ---
    // Only the Settings tab has normal mode keybindings for now.
    if matches!(app.active_tab, Tab::Settings) {
        handle_settings_keys(key.code, app, terminal).await?;
    }

    Ok(())
}

/// Handles key presses for the Settings tab.
async fn handle_settings_keys(
    key_code: KeyCode,
    app: &mut App,
    terminal: &mut TerminalBackend,
) -> Result<()> {
    match key_code {
        KeyCode::Char('x') if matches!(app.auth_state, AuthState::Authenticated { .. }) => {
            if let Err(e) = app.logout() {
                app.status = format!("Logout failed: {e}");
            } else {
                // After logging out (becoming a guest), refresh the documents.
                app.fetch_documents().await;
            }
        }
        KeyCode::Char('l') if app.auth_state == AuthState::Guest => {
            // This allows a guest user to log in from the settings page.
            app.status = "Logging in via browser...".to_string();
            // We need to draw the UI here to show the status message before the blocking login call.
            terminal.draw(|f| ui(f, app))?;

            match auth::login().await {
                Ok(token) => {
                    if let Err(e) = app.login(&token).await {
                        app.status = format!("Failed to store token: {e}");
                    } else {
                        // After a successful login, fetch the user's documents.
                        app.fetch_documents().await;
                    }
                }
                Err(e) => {
                    app.status = format!("Login failed: {e}");
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Handles key presses when in `InputMode::Editing`.
async fn handle_editing_keys(key: KeyEvent, app: &mut App) -> Result<()> {
    match key.code {
        // Enter submits the input from the text box.
        KeyCode::Enter => {
            app.submit_ingestion().await;
            // The submit_ingestion function handles changing the mode back to Normal.
        }
        // Esc quits the application.
        KeyCode::Esc => {
            app.quit();
        }
        // Backspace removes the last character.
        KeyCode::Backspace => {
            app.input_text.pop();
        }
        // Any other character is appended to the input text.
        KeyCode::Char(c) => {
            app.input_text.push(c);
        }
        _ => {}
    }
    Ok(())
}

// --- Terminal Setup and Restoration ---

/// Sets up the terminal for TUI rendering.
fn setup_terminal() -> Result<TerminalBackend> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restores the terminal to its original state after the TUI exits.
fn restore_terminal(terminal: &mut TerminalBackend) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableBracketedPaste
    )?;
    terminal.show_cursor()?;
    Ok(())
}
