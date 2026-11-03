//! TUI module for IntelliWallet CLI
//!
//! Midnight Commander style dual-panel interface.

mod app;
mod dialogs;
mod events;
mod panels;
mod ui;

use std::io;
use std::path::Path;

use ratatui::crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;

pub use app::App;

/// Run the TUI application
pub fn run(wallet_path: Option<&str>) -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(wallet_path.map(Path::new));

    // Run main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

/// Main application loop
fn run_app<B: Backend<Error = io::Error>>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    let mut mouse_captured = true;

    loop {
        // Toggle mouse capture based on dialog state
        let in_field_details = matches!(
            &app.mode,
            app::AppMode::Dialog(app::DialogType::FieldDetails)
        );

        if in_field_details && mouse_captured {
            // Disable mouse capture to allow text selection in field details
            execute!(io::stdout(), DisableMouseCapture)?;
            mouse_captured = false;
        } else if !in_field_details && !mouse_captured {
            // Re-enable mouse capture when leaving field details
            execute!(io::stdout(), EnableMouseCapture)?;
            mouse_captured = true;
        }

        terminal.draw(|f| ui::draw(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                if events::handle_key(app, key.code) {
                    return Ok(());
                }
            }
        }
    }
}

/// Show about screen (when no wallet path provided)
pub fn show_about() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.draw(|f| {
        let area = f.area();
        let block = ratatui::widgets::Block::default()
            .title(" IntelliWallet CLI ")
            .borders(ratatui::widgets::Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded);

        let inner = block.inner(area);
        f.render_widget(block, area);

        let text = vec![
            Line::from(""),
            Line::from(format!(
                "  IntelliWallet CLI v{}",
                env!("CARGO_PKG_VERSION")
            )),
            Line::from("  Password Manager for Terminal"),
            Line::from(""),
            Line::from("  Usage:"),
            Line::from("    iwcli --wallet <path>"),
            Line::from(""),
            Line::from("  Example:"),
            Line::from("    iwcli --wallet ~/MyWallet"),
            Line::from(""),
            Line::from("  Press any key to exit..."),
        ];

        let paragraph = ratatui::widgets::Paragraph::new(text);
        f.render_widget(paragraph, inner);
    })?;

    // Wait for any key
    loop {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                break;
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
