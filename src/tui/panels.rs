//! Panel widgets for left (folders/items) and right (fields) panels

use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, List, ListItem},
    Frame,
};
use totp_rs::{Algorithm, TOTP};

use super::app::{App, Panel};

/// Generate TOTP code from a secret. Returns (code, seconds_remaining)
pub fn generate_totp(secret: &str) -> Option<(String, u64)> {
    use base32::Alphabet;

    // Try to parse as otpauth:// URL first, then as raw base32 secret
    let totp = if secret.starts_with("otpauth://") {
        TOTP::from_url(secret).ok()?
    } else {
        // Assume raw base32 secret - decode it properly
        let secret_clean = secret.replace(' ', "").to_uppercase();
        let decoded = base32::decode(Alphabet::Rfc4648 { padding: false }, &secret_clean)?;
        TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            decoded,
            None,
            "".to_string(),
        ).ok()?
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs();

    let code = totp.generate(now);
    let seconds_remaining = 30 - (now % 30);

    Some((code, seconds_remaining))
}

/// Draw the left panel (folder contents)
pub fn draw_left_panel(frame: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::Left;

    // Build title with current folder name
    let title = format!(" {} ", app.current_folder_name);

    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(if is_active {
            BorderType::Thick
        } else {
            BorderType::Rounded
        })
        .style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build list items
    let mut items: Vec<ListItem> = Vec::new();

    // Add ".." if not in root
    if !app.is_root() {
        let style = if is_active && app.left_selected == 0 {
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Yellow)
        };
        items.push(ListItem::new(Line::from(Span::styled("..", style))));
    }

    // Add folder items
    for (i, item) in app.folder_items.iter().enumerate() {
        let actual_index = if app.is_root() { i } else { i + 1 };
        let is_selected = is_active && app.left_selected == actual_index;

        let style = if is_selected {
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else if item.folder {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        items.push(ListItem::new(Line::from(Span::styled(item.name.clone(), style))));
    }

    // If empty, show hint
    if items.is_empty() {
        items.push(ListItem::new(
            Line::from(Span::styled(
                "(empty)",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            ))
        ));
    }

    // Calculate scroll offset to keep selected item visible
    let visible_height = inner.height as usize;
    let total_items = items.len();
    let selected = app.left_selected;

    let scroll_offset = if total_items <= visible_height {
        0
    } else if selected < visible_height / 2 {
        0
    } else if selected >= total_items.saturating_sub(visible_height / 2) {
        total_items.saturating_sub(visible_height)
    } else {
        selected.saturating_sub(visible_height / 2)
    };

    // Take only visible items
    let visible_items: Vec<ListItem> = items
        .into_iter()
        .skip(scroll_offset)
        .take(visible_height)
        .collect();

    let list = List::new(visible_items);
    frame.render_widget(list, inner);
}

/// Draw the right panel (fields)
pub fn draw_right_panel(frame: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::Right;

    let title = if let Some(item) = &app.selected_item {
        if item.folder {
            " Folder ".to_string()
        } else {
            " Fields ".to_string()
        }
    } else {
        " Fields ".to_string()
    };

    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(if is_active {
            BorderType::Thick
        } else {
            BorderType::Rounded
        })
        .style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build list items
    let mut items: Vec<ListItem> = Vec::new();

    if let Some(selected_item) = &app.selected_item {
        if selected_item.folder {
            // Show folder info
            items.push(ListItem::new(Line::from(Span::styled(
                format!("Folder: {}", selected_item.name),
                Style::default().fg(Color::Yellow),
            ))));
            items.push(ListItem::new(Line::from(Span::styled(
                "Press Enter to open",
                Style::default().fg(Color::DarkGray),
            ))));
        } else {
            // Show fields as "Label: Value"
            for (i, field) in app.fields.iter().enumerate() {
                let is_selected = is_active && app.right_selected == i;

                let style = if is_selected {
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                // Check if this is a 2FA field
                let display_value = if field.field_type == "2FAC" {
                    if let Some((code, remaining)) = generate_totp(&field.value) {
                        format!("{} ({}s)", code, remaining)
                    } else {
                        field.value.clone()
                    }
                } else {
                    field.value.clone()
                };

                let display_text = format!("{}: {}", field.label, display_value);
                items.push(ListItem::new(Line::from(Span::styled(display_text, style))));
            }

            if app.fields.is_empty() {
                items.push(ListItem::new(Line::from(Span::styled(
                    "(no fields)",
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                ))));
                items.push(ListItem::new(Line::from(Span::styled(
                    "Press F3 to add field",
                    Style::default().fg(Color::DarkGray),
                ))));
            }
        }
    } else {
        // No item selected
        items.push(ListItem::new(Line::from(Span::styled(
            "Select an item",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        ))));
    }

    // Calculate scroll offset to keep selected item visible
    let visible_height = inner.height as usize;
    let total_items = items.len();
    let selected = app.right_selected;

    let scroll_offset = if total_items <= visible_height {
        0
    } else if selected < visible_height / 2 {
        0
    } else if selected >= total_items.saturating_sub(visible_height / 2) {
        total_items.saturating_sub(visible_height)
    } else {
        selected.saturating_sub(visible_height / 2)
    };

    // Take only visible items
    let visible_items: Vec<ListItem> = items
        .into_iter()
        .skip(scroll_offset)
        .take(visible_height)
        .collect();

    let list = List::new(visible_items);
    frame.render_widget(list, inner);
}
