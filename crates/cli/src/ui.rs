//! # TUI Rendering Logic
//!
//! This module is responsible for drawing the entire user interface based on the
//! current application state. It features a persistent input panel at the top.

use crate::app::{App, AuthState, InputMode, Tab};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs},
};

/// The main rendering function, which draws the unified layout.
pub fn ui(frame: &mut Frame, app: &App) {
    // This is the main layout for the application.
    // It's a vertical layout with four sections:
    // 1. Input Panel (always visible)
    // 2. Tabs for navigation
    // 3. Main content area for the active tab
    // 4. Status bar at the bottom
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // For Input Panel
            Constraint::Length(3), // For Tabs
            Constraint::Min(0),    // Main content
            Constraint::Length(1), // Status bar
        ])
        .split(frame.size());

    // Render each part of the UI into its designated area.
    render_input_panel(frame, app, main_layout[0]);
    render_tabs(frame, app, main_layout[1]);
    render_main_content(frame, app, main_layout[2]);
    render_status_bar(frame, app, main_layout[3]);
}

/// Renders the top input panel for URL or file path ingestion.
fn render_input_panel(frame: &mut Frame, app: &App, area: Rect) {
    let outer_block = Block::default().title("INPUT").borders(Borders::ALL);
    let inner_area = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(10)]) // Input area and submit button
        .split(inner_area);

    let input = Paragraph::new(app.input_text.as_str()).style(Style::default().fg(Color::Yellow));
    frame.render_widget(input, chunks[0]);

    let submit_button = Paragraph::new("[ Submit ]")
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);
    frame.render_widget(submit_button, chunks[1]);

    // Only show the cursor when in Editing mode.
    if app.input_mode == InputMode::Editing {
        frame.set_cursor(chunks[0].x + app.input_text.len() as u16, chunks[0].y);
    }
}

/// Renders the top navigation tabs.
fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let mut titles = vec!["DB"];
    if app.get_role() == "root" {
        titles.push("Users");
    }
    titles.push("Settings");

    let selected_index = match app.active_tab {
        Tab::Db => 0,
        Tab::Users => titles.iter().position(|&t| t == "Users").unwrap_or(0),
        Tab::Settings => titles.len() - 1,
    };

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::BOTTOM))
        .select(selected_index)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        );

    frame.render_widget(tabs, area);
}

/// Renders the main content area based on the active tab.
fn render_main_content(frame: &mut Frame, app: &App, area: Rect) {
    let content_block = Block::default().title("Content").borders(Borders::ALL);
    let inner_area = content_block.inner(area);
    frame.render_widget(content_block, area);

    match app.active_tab {
        Tab::Db => render_db_tab(frame, app, inner_area),
        Tab::Users => render_users_tab(frame, app, inner_area),
        Tab::Settings => render_settings_tab(frame, app, inner_area),
    }
}

/// Renders the content for the "DB" tab, showing a table of documents.
fn render_db_tab(frame: &mut Frame, app: &App, area: Rect) {
    let header_cells = ["Title", "Source URL", "Owner ID", "Created At"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow)));
    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::DarkGray))
        .height(1);

    let rows = app.documents.iter().map(|doc| {
        let cells = vec![
            Cell::from(doc.title.clone()),
            Cell::from(doc.source_url.clone()),
            Cell::from(doc.owner_id.clone()),
            Cell::from(doc.created_at.clone()),
        ];
        Row::new(cells)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
        ],
    )
    .header(header)
    .block(Block::default().title("Documents"));

    frame.render_widget(table, area);
}

/// Renders the content for the "Users" tab, showing a table of users.
fn render_users_tab(frame: &mut Frame, app: &App, area: Rect) {
    let header_cells = ["ID", "Role", "Created At"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow)));
    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::DarkGray))
        .height(1);

    let rows = app.users.iter().map(|user| {
        let cells = vec![
            Cell::from(user.id.clone()),
            Cell::from(user.role.clone()),
            Cell::from(user.created_at.clone()),
        ];
        Row::new(cells)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(50),
            Constraint::Percentage(10),
            Constraint::Percentage(40),
        ],
    )
    .header(header)
    .block(Block::default().title("Users"));

    frame.render_widget(table, area);
}

/// Renders the content for the "Settings" tab.
fn render_settings_tab(frame: &mut Frame, app: &App, area: Rect) {
    let mut items: Vec<String> = vec!["".into()]; // Add a top margin

    match app.auth_state {
        AuthState::Authenticated { .. } => {
            items.push("  Logout (Press 'x')".into());
        }
        AuthState::Guest => {
            items.push("  Login with Google (Press 'l')".into());
        }
    }
    items.push("  Quit (Press 'q')".into());

    let list = Paragraph::new(items.join("\n"));

    frame.render_widget(list, area);
}

/// Renders the bottom status bar.
fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let status = Paragraph::new(app.status.as_str())
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));
    frame.render_widget(status, area);
}
