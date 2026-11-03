//! UI layout and rendering

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use super::app::{App, AppMode};
use super::dialogs;
use super::panels;

/// Main draw function
pub fn draw(frame: &mut Frame, app: &App) {
    match &app.mode {
        AppMode::Password => draw_password_screen(frame, app),
        AppMode::CreateWallet => draw_create_wallet_screen(frame, app),
        AppMode::Browse => draw_main_screen(frame, app),
        AppMode::Dialog(dialog_type) => {
            // Draw main screen behind dialog
            draw_main_screen(frame, app);
            // Draw dialog on top
            dialogs::draw_dialog(frame, app, dialog_type);
        }
    }
}

/// Draw password entry screen
fn draw_password_screen(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Center the password dialog
    let dialog_width = 50;
    let dialog_height = 10;
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(
        x,
        y,
        dialog_width.min(area.width),
        dialog_height.min(area.height),
    );

    // Clear background
    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" Unlock Wallet ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Cyan));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    // Layout inside dialog
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Path
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Password label
            Constraint::Length(1), // Password input
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Error or hint
        ])
        .split(inner);

    // Wallet path
    let path = app
        .wallet_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let path_text =
        Paragraph::new(format!("Wallet: {}", path)).style(Style::default().fg(Color::Gray));
    frame.render_widget(path_text, chunks[0]);

    // Password label
    let label = Paragraph::new("Password:").style(Style::default().fg(Color::White));
    frame.render_widget(label, chunks[2]);

    // Password input (masked)
    let masked: String = "*".repeat(app.password_input.len());
    let input = Paragraph::new(format!("[{}]", masked)).style(Style::default().fg(Color::Yellow));
    frame.render_widget(input, chunks[3]);

    // Error message or hint
    if let Some(error) = &app.password_error {
        let error_text = Paragraph::new(error.as_str()).style(Style::default().fg(Color::Red));
        frame.render_widget(error_text, chunks[5]);
    } else {
        let hint = Paragraph::new("Enter: unlock | Esc/Q: quit")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(hint, chunks[5]);
    }
}

/// Draw create wallet screen
fn draw_create_wallet_screen(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Center the dialog
    let dialog_width = 50;
    let dialog_height = 14;
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(
        x,
        y,
        dialog_width.min(area.width),
        dialog_height.min(area.height),
    );

    // Clear background
    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" Create New Wallet ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Green));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    // Layout inside dialog
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Path
            Constraint::Length(1), // Info
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Password label
            Constraint::Length(1), // Password input
            Constraint::Length(1), // Confirm label
            Constraint::Length(1), // Confirm input
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Error or hint
        ])
        .split(inner);

    // Wallet path
    let path = app
        .wallet_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let path_text =
        Paragraph::new(format!("Location: {}", path)).style(Style::default().fg(Color::Gray));
    frame.render_widget(path_text, chunks[0]);

    // Info message
    let info = Paragraph::new("Wallet not found. Create a new one?")
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(info, chunks[1]);

    // Password label
    let pwd_style = if app.create_wallet_focus == 0 {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };
    let label = Paragraph::new("Password:").style(pwd_style);
    frame.render_widget(label, chunks[3]);

    // Password input (masked)
    let masked: String = "*".repeat(app.password_input.len());
    let input_style = if app.create_wallet_focus == 0 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let input = Paragraph::new(format!("[{}]", masked)).style(input_style);
    frame.render_widget(input, chunks[4]);

    // Confirm label
    let confirm_style = if app.create_wallet_focus == 1 {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };
    let confirm_label = Paragraph::new("Confirm:").style(confirm_style);
    frame.render_widget(confirm_label, chunks[5]);

    // Confirm input (masked)
    let confirm_masked: String = "*".repeat(app.password_confirm.len());
    let confirm_input_style = if app.create_wallet_focus == 1 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let confirm_input = Paragraph::new(format!("[{}]", confirm_masked)).style(confirm_input_style);
    frame.render_widget(confirm_input, chunks[6]);

    // Error message or hint
    if let Some(error) = &app.password_error {
        let error_text = Paragraph::new(error.as_str()).style(Style::default().fg(Color::Red));
        frame.render_widget(error_text, chunks[8]);
    } else {
        let hint = Paragraph::new("Tab: switch | Enter: create | Esc/Q: quit")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(hint, chunks[8]);
    }
}

/// Draw main browsing screen
fn draw_main_screen(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Main layout: header, content, footer
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(5),    // Content (panels)
            Constraint::Length(1), // Footer (toolbar)
        ])
        .split(area);

    // Draw header
    draw_header(frame, app, main_chunks[0]);

    // Draw panels
    let panel_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[1]);

    panels::draw_left_panel(frame, app, panel_chunks[0]);
    panels::draw_right_panel(frame, app, panel_chunks[1]);

    // Draw footer/toolbar
    draw_toolbar(frame, app, main_chunks[2]);
}

/// Draw header with path and status
fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" IntelliWallet ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build path with selected item name
    let mut path = app.current_path();
    if let Some(item) = &app.selected_item {
        path.push_str(&item.name);
    }

    // Show selected item's icon on the right
    let icon = app
        .selected_item
        .as_ref()
        .map(|item| format!("[{}]", item.icon))
        .unwrap_or_default();

    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(20)])
        .split(inner);

    let path_text = Paragraph::new(path).style(Style::default().fg(Color::White));
    frame.render_widget(path_text, content[0]);

    let icon_text = Paragraph::new(icon)
        .style(Style::default().fg(Color::Yellow))
        .alignment(ratatui::layout::Alignment::Right);
    frame.render_widget(icon_text, content[1]);
}

/// Draw toolbar at bottom
fn draw_toolbar(frame: &mut Frame, app: &App, area: Rect) {
    use super::app::Panel;

    // N label changes based on active panel
    let new_label = match app.active_panel {
        Panel::Left => ":New Item ",
        Panel::Right => ":New Field ",
    };

    let spans = vec![
        Span::styled(" N", Style::default().fg(Color::Black).bg(Color::Cyan)),
        Span::raw(new_label),
        Span::styled(" E", Style::default().fg(Color::Black).bg(Color::Cyan)),
        Span::raw(":Edit "),
        Span::styled(" C", Style::default().fg(Color::Black).bg(Color::Cyan)),
        Span::raw(":New Folder "),
        Span::styled(" D", Style::default().fg(Color::Black).bg(Color::Cyan)),
        Span::raw(":Del "),
        Span::styled(" P", Style::default().fg(Color::Black).bg(Color::Cyan)),
        Span::raw(":Change Password "),
        Span::styled(" Q", Style::default().fg(Color::Black).bg(Color::Cyan)),
        Span::raw(":Quit "),
    ];

    let toolbar = Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::DarkGray));
    frame.render_widget(toolbar, area);
}
