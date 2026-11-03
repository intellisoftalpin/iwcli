//! Panel widgets for left (folders/items) and right (fields) panels

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem},
};

use super::app::{App, Panel};

/// First list row to draw so that `selected` stays visible, roughly centred.
pub fn scroll_offset(total_items: usize, visible_height: usize, selected: usize) -> usize {
    if total_items <= visible_height || selected < visible_height / 2 {
        0
    } else if selected >= total_items.saturating_sub(visible_height / 2) {
        total_items.saturating_sub(visible_height)
    } else {
        selected.saturating_sub(visible_height / 2)
    }
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

        items.push(ListItem::new(Line::from(Span::styled(
            item.name.clone(),
            style,
        ))));
    }

    // If empty, show hint
    if items.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "(empty)",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ))));
    }

    // Calculate scroll offset to keep selected item visible
    let visible_height = inner.height as usize;
    let total_items = items.len();
    let selected = app.left_selected;

    let scroll_offset = scroll_offset(total_items, visible_height, selected);

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

                // For 2FA fields, show raw secret in panel (TOTP code shown in popup dialog)
                let display_value = field.value.clone();

                let display_text = format!("{}: {}", field.label, display_value);
                items.push(ListItem::new(Line::from(Span::styled(display_text, style))));
            }

            if app.fields.is_empty() {
                items.push(ListItem::new(Line::from(Span::styled(
                    "(no fields)",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
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
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ))));
    }

    // Calculate scroll offset to keep selected item visible
    let visible_height = inner.height as usize;
    let total_items = items.len();
    let selected = app.right_selected;

    let scroll_offset = scroll_offset(total_items, visible_height, selected);

    // Take only visible items
    let visible_items: Vec<ListItem> = items
        .into_iter()
        .skip(scroll_offset)
        .take(visible_height)
        .collect();

    let list = List::new(visible_items);
    frame.render_widget(list, inner);
}

#[cfg(test)]
mod tests {
    use super::scroll_offset;

    #[test]
    fn no_scroll_when_everything_fits() {
        assert_eq!(scroll_offset(5, 10, 4), 0);
        assert_eq!(scroll_offset(10, 10, 9), 0);
    }

    #[test]
    fn no_scroll_near_the_top() {
        assert_eq!(scroll_offset(100, 10, 0), 0);
        assert_eq!(scroll_offset(100, 10, 4), 0);
    }

    #[test]
    fn selection_is_centred_in_the_middle() {
        assert_eq!(scroll_offset(100, 10, 5), 0);
        assert_eq!(scroll_offset(100, 10, 50), 45);
    }

    #[test]
    fn last_page_is_pinned_at_the_end() {
        assert_eq!(scroll_offset(100, 10, 95), 90);
        assert_eq!(scroll_offset(100, 10, 99), 90);
    }

    #[test]
    fn zero_height_does_not_panic() {
        assert_eq!(scroll_offset(3, 0, 2), 2);
    }
}
