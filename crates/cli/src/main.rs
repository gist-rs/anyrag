//! # anyrag-cli: An Interactive TUI for `anyrag`
//!
//! This is the main entry point for the `anyrag` terminal user interface.

mod auth;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use keyring::Entry;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use std::io::{self, Stdout};
use std::time::Duration;
use tracing_subscriber::FmtSubscriber;

const KEYRING_SERVICE: &str = "anyrag-cli";
const KEYRING_USERNAME: &str = "user";

// --- Application State ---

/// Represents the state of the application.
struct App {
    running: bool,
    authenticated: bool,
    status: String,
    keyring_entry: Entry,
}

impl App {
    /// Creates a new instance of the application state.
    fn new() -> Result<Self> {
        let entry = Entry::new(KEYRING_SERVICE, KEYRING_USERNAME)?;
        let authenticated = entry.get_password().is_ok();
        let status = if authenticated {
            "Authenticated. Press 'q' to quit.".to_string()
        } else {
            "Press 'l' to log in or 'q' to quit.".to_string()
        };

        Ok(Self {
            running: true,
            authenticated,
            status,
            keyring_entry: entry,
        })
    }

    /// Sets the `running` flag to false to exit the main loop.
    fn quit(&mut self) {
        self.running = false;
    }
}

// --- Main Application Entry ---

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging (optional, but helpful for debugging)
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Setup terminal
    let mut terminal = setup_terminal()?;

    // Create app instance and run the main TUI loop
    let mut app = App::new()?;
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
        // Draw the current state of the UI
        terminal.draw(|f| ui(f, app))?;

        // Handle user input and other events
        // Poll for an event with a timeout of 250ms
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => app.quit(),
                        KeyCode::Char('l') if !app.authenticated => {
                            // Update status and redraw immediately to give user feedback.
                            app.status = "Logging in via browser...".to_string();
                            terminal.draw(|f| ui(f, app))?;

                            match auth::login().await {
                                Ok(token) => {
                                    if let Err(e) = app.keyring_entry.set_password(&token) {
                                        app.status = format!("Failed to store token: {e}");
                                    } else {
                                        app.authenticated = true;
                                        app.status = "Login successful! You can now manage the knowledge base.".to_string();
                                    }
                                }
                                Err(e) => {
                                    app.status = format!("Login failed: {e}");
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

/// Renders the user interface.
fn ui(frame: &mut Frame, app: &App) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(100)])
        .split(frame.size());

    let content = Paragraph::new(app.status.as_str())
        .block(Block::default().title("anyrag-cli").borders(Borders::ALL))
        .alignment(Alignment::Center);

    frame.render_widget(content, main_layout[0]);
}

// --- Terminal Setup and Restoration ---

/// Sets up the terminal for TUI rendering.
fn setup_terminal() -> Result<TerminalBackend> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
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
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
