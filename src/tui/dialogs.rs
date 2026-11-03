//! Modal dialog widgets

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
};

use super::app::{App, DialogType};
use crate::totp::generate_totp;

/// Draw a dialog based on type
pub fn draw_dialog(frame: &mut Frame, app: &App, dialog_type: &DialogType) {
    match dialog_type {
        DialogType::NewItem | DialogType::NewFolder => draw_new_item_dialog(frame, app),
        DialogType::NewField => draw_new_field_dialog(frame, app),
        DialogType::EditItem => draw_edit_item_dialog(frame, app),
        DialogType::EditField => draw_edit_field_dialog(frame, app),
        DialogType::DeleteItem => draw_delete_item_dialog(frame, app),
        DialogType::DeleteField => draw_delete_field_dialog(frame, app),
        DialogType::FieldDetails => draw_field_details_dialog(frame, app),
        DialogType::ChangePassword => draw_change_password_dialog(frame, app),
    }
}

/// Draw new item/folder dialog
fn draw_new_item_dialog(frame: &mut Frame, app: &App) {
    let area = frame.area();

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

    frame.render_widget(Clear, dialog_area);

    let title = if app.creating_folder {
        " New Folder "
    } else {
        " New Item "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Yellow));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Name label
            Constraint::Length(1), // Name input
            Constraint::Length(1), // Icon label
            Constraint::Length(1), // Icon input
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Hint
        ])
        .split(inner);

    // Name label
    let label = Paragraph::new("Name:").style(Style::default().fg(Color::White));
    frame.render_widget(label, chunks[0]);

    // Name input
    let input = Paragraph::new(format!("[{}]", &app.input_buffer))
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(input, chunks[1]);

    // Icon label
    let icon_label = Paragraph::new("Icon:").style(Style::default().fg(Color::White));
    frame.render_widget(icon_label, chunks[2]);

    // Icon input
    let icon_hint = if app.input_secondary.is_empty() {
        "folder"
    } else {
        &app.input_secondary
    };
    let icon_input =
        Paragraph::new(format!("[{}]", icon_hint)).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(icon_input, chunks[3]);

    // Hint
    let hint =
        Paragraph::new("Enter: Create  |  Esc: Cancel").style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hint, chunks[5]);
}

/// Draw new field dialog with label selection
fn draw_new_field_dialog(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let dialog_width = 60;
    let dialog_height = 16;
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(
        x,
        y,
        dialog_width.min(area.width),
        dialog_height.min(area.height),
    );

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" New Field ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Yellow));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Type label
            Constraint::Length(8), // Type list
            Constraint::Length(1), // Value label
            Constraint::Length(1), // Value input
            Constraint::Length(1), // Hint
        ])
        .split(inner);

    // Type label
    let label =
        Paragraph::new("Field Type (Up/Down to select):").style(Style::default().fg(Color::White));
    frame.render_widget(label, chunks[0]);

    // Type list (show subset of labels)
    let visible_labels: Vec<ListItem> = app
        .labels
        .iter()
        .enumerate()
        .skip(app.dialog_selected.saturating_sub(3))
        .take(6)
        .map(|(i, label)| {
            let is_selected = i == app.dialog_selected;
            let style = if is_selected {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(
                format!("[{}] {}", label.icon, label.name),
                style,
            )))
        })
        .collect();

    let list = List::new(visible_labels);
    frame.render_widget(list, chunks[1]);

    // Value label
    let value_label = Paragraph::new("Value:").style(Style::default().fg(Color::White));
    frame.render_widget(value_label, chunks[2]);

    // Value input
    let input = Paragraph::new(format!("[{}]", &app.input_buffer))
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(input, chunks[3]);

    // Hint
    let hint =
        Paragraph::new("Enter: Create  |  Esc: Cancel").style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hint, chunks[4]);
}

/// Draw delete item confirmation dialog
fn draw_delete_item_dialog(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let dialog_width = 50;
    let dialog_height = 8;
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(
        x,
        y,
        dialog_width.min(area.width),
        dialog_height.min(area.height),
    );

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" Delete Item ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Red));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Message
            Constraint::Length(1), // Item name
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Hint
        ])
        .split(inner);

    let message = Paragraph::new("Delete this item?").style(Style::default().fg(Color::White));
    frame.render_widget(message, chunks[0]);

    let item_name = app
        .selected_item
        .as_ref()
        .map(|i| i.name.clone())
        .unwrap_or_default();
    let name_text = Paragraph::new(item_name).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(name_text, chunks[1]);

    let hint =
        Paragraph::new("Enter: Delete  |  Esc: Cancel").style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hint, chunks[3]);
}

/// Draw delete field confirmation dialog
fn draw_delete_field_dialog(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let dialog_width = 50;
    let dialog_height = 8;
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(
        x,
        y,
        dialog_width.min(area.width),
        dialog_height.min(area.height),
    );

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" Delete Field ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Red));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Message
            Constraint::Length(1), // Field info
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Hint
        ])
        .split(inner);

    let message = Paragraph::new("Delete this field?").style(Style::default().fg(Color::White));
    frame.render_widget(message, chunks[0]);

    let field_info = if let Some(field) = app.fields.get(app.right_selected) {
        format!("{}: {}", field.label, field.value)
    } else {
        String::new()
    };
    let field_text = Paragraph::new(field_info).style(Style::default().fg(Color::Yellow));
    frame.render_widget(field_text, chunks[1]);

    let hint =
        Paragraph::new("Enter: Delete  |  Esc: Cancel").style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hint, chunks[3]);
}

/// Draw edit item dialog
fn draw_edit_item_dialog(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let dialog_width = 50;
    let dialog_height = 8;
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(
        x,
        y,
        dialog_width.min(area.width),
        dialog_height.min(area.height),
    );

    frame.render_widget(Clear, dialog_area);

    let title = if app
        .selected_item
        .as_ref()
        .map(|i| i.folder)
        .unwrap_or(false)
    {
        " Edit Folder "
    } else {
        " Edit Item "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Yellow));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Name label
            Constraint::Length(1), // Name input
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Hint
        ])
        .split(inner);

    // Name label
    let label = Paragraph::new("Name:").style(Style::default().fg(Color::White));
    frame.render_widget(label, chunks[0]);

    // Name input
    let input = Paragraph::new(format!("[{}]", &app.input_buffer))
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(input, chunks[1]);

    // Hint
    let hint =
        Paragraph::new("Enter: Save  |  Esc: Cancel").style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hint, chunks[3]);
}

/// Draw edit field dialog
fn draw_edit_field_dialog(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let dialog_width = 60;
    let dialog_height = 10;
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(
        x,
        y,
        dialog_width.min(area.width),
        dialog_height.min(area.height),
    );

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" Edit Field ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Yellow));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Field info
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Value label
            Constraint::Length(1), // Value input
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Hint
        ])
        .split(inner);

    // Field info
    let field_label = app
        .fields
        .get(app.right_selected)
        .map(|f| format!("[{}] {}", f.icon, f.label))
        .unwrap_or_default();
    let info = Paragraph::new(field_label).style(Style::default().fg(Color::Cyan));
    frame.render_widget(info, chunks[0]);

    // Value label
    let label = Paragraph::new("Value:").style(Style::default().fg(Color::White));
    frame.render_widget(label, chunks[2]);

    // Value input
    let input = Paragraph::new(format!("[{}]", &app.input_buffer))
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(input, chunks[3]);

    // Hint
    let hint =
        Paragraph::new("Enter: Save  |  Esc: Cancel").style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hint, chunks[5]);
}

/// Draw field details dialog
fn draw_field_details_dialog(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let dialog_width = 60;
    let dialog_height = 14;
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(
        x,
        y,
        dialog_width.min(area.width),
        dialog_height.min(area.height),
    );

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" Field Details ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Cyan));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    if let Some(field) = app.fields.get(app.right_selected) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(1), // Label
                Constraint::Length(1), // Field type
                Constraint::Length(1), // Icon
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Value label
                Constraint::Length(2), // Value
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Changed
                Constraint::Length(1), // Hint
            ])
            .split(inner);

        // Label name
        let label_text = Paragraph::new(format!("Label: {}", field.label))
            .style(Style::default().fg(Color::White));
        frame.render_widget(label_text, chunks[0]);

        // Field type
        let type_text =
            Paragraph::new(format!("Type: {} ({})", field.field_type, field.value_type))
                .style(Style::default().fg(Color::White));
        frame.render_widget(type_text, chunks[1]);

        // Icon
        let icon_text = Paragraph::new(format!("Icon: [{}]", field.icon))
            .style(Style::default().fg(Color::White));
        frame.render_widget(icon_text, chunks[2]);

        // Value (show TOTP code for 2FA fields)
        let (value_label_text, display_value) = if field.field_type == "2FAC" {
            if let Some((code, remaining)) = generate_totp(&field.value) {
                ("TOTP Code:", format!("{} ({}s remaining)", code, remaining))
            } else {
                ("Value:", field.value.clone())
            }
        } else {
            ("Value:", field.value.clone())
        };

        let value_label =
            Paragraph::new(value_label_text).style(Style::default().fg(Color::DarkGray));
        frame.render_widget(value_label, chunks[4]);

        let value_text = Paragraph::new(display_value).style(Style::default().fg(Color::Yellow));
        frame.render_widget(value_text, chunks[5]);

        // Changed timestamp
        let changed = field
            .change_timestamp
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let changed_text = Paragraph::new(format!("Changed: {}", changed))
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(changed_text, chunks[7]);

        // Hint
        let hint = Paragraph::new("Press Enter or Esc to close")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(hint, chunks[8]);
    }
}

/// Draw change password dialog
fn draw_change_password_dialog(frame: &mut Frame, app: &App) {
    let area = frame.area();

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

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" Change Password ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Yellow));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Current password label
            Constraint::Length(1), // Current password input
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // New password label
            Constraint::Length(1), // New password input
            Constraint::Length(1), // Confirm label
            Constraint::Length(1), // Confirm input
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Error or hint
        ])
        .split(inner);

    // Helper to get style based on focus
    let focused = |idx: usize| {
        if app.change_password_focus == idx {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        }
    };
    let input_focused = |idx: usize| {
        if app.change_password_focus == idx {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        }
    };

    // Current password
    let label = Paragraph::new("Current Password:").style(focused(0));
    frame.render_widget(label, chunks[0]);

    let masked: String = "*".repeat(app.old_password.len());
    let input = Paragraph::new(format!("[{}]", masked)).style(input_focused(0));
    frame.render_widget(input, chunks[1]);

    // New password
    let new_label = Paragraph::new("New Password:").style(focused(1));
    frame.render_widget(new_label, chunks[3]);

    let new_masked: String = "*".repeat(app.new_password.len());
    let new_input = Paragraph::new(format!("[{}]", new_masked)).style(input_focused(1));
    frame.render_widget(new_input, chunks[4]);

    // Confirm new password
    let confirm_label = Paragraph::new("Confirm New Password:").style(focused(2));
    frame.render_widget(confirm_label, chunks[5]);

    let confirm_masked: String = "*".repeat(app.new_password_confirm.len());
    let confirm_input = Paragraph::new(format!("[{}]", confirm_masked)).style(input_focused(2));
    frame.render_widget(confirm_input, chunks[6]);

    // Error or hint
    if let Some(error) = &app.password_error {
        let error_text = Paragraph::new(error.as_str()).style(Style::default().fg(Color::Red));
        frame.render_widget(error_text, chunks[8]);
    } else {
        let hint = Paragraph::new("Tab/Up/Down: switch | Enter: change | Esc: cancel")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(hint, chunks[8]);
    }
}
