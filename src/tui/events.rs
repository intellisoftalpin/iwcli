//! Keyboard event handling

use ratatui::crossterm::event::KeyCode;

use super::app::{App, AppMode, DialogType, Panel};

/// Handle a key press. Returns true if app should quit.
pub fn handle_key(app: &mut App, key: KeyCode) -> bool {
    match &app.mode {
        AppMode::Password => handle_password_key(app, key),
        AppMode::CreateWallet => handle_create_wallet_key(app, key),
        AppMode::Browse => handle_browse_key(app, key),
        AppMode::Dialog(dialog_type) => handle_dialog_key(app, key, dialog_type.clone()),
    }
}

/// Handle keys in password entry mode
fn handle_password_key(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => true, // Quit
        KeyCode::Enter => {
            app.try_unlock();
            false
        }
        KeyCode::Backspace => {
            app.password_input.pop();
            false
        }
        KeyCode::Char(c) => {
            app.password_input.push(c);
            false
        }
        _ => false,
    }
}

/// Handle keys in create wallet mode
fn handle_create_wallet_key(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => true, // Quit
        KeyCode::Enter => {
            app.try_create_wallet();
            false
        }
        KeyCode::Tab | KeyCode::Down | KeyCode::Up => {
            // Toggle between password and confirm fields
            app.create_wallet_focus = if app.create_wallet_focus == 0 { 1 } else { 0 };
            false
        }
        KeyCode::Backspace => {
            if app.create_wallet_focus == 0 {
                app.password_input.pop();
            } else {
                app.password_confirm.pop();
            }
            false
        }
        KeyCode::Char(c) => {
            if app.create_wallet_focus == 0 {
                app.password_input.push(c);
            } else {
                app.password_confirm.push(c);
            }
            false
        }
        _ => false,
    }
}

/// Handle keys in browse mode
fn handle_browse_key(app: &mut App, key: KeyCode) -> bool {
    // Clear any previous messages
    app.message = None;
    app.error = None;

    match key {
        // Quit
        KeyCode::Char('q') => true,

        // Navigation
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_up();
            false
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_down();
            false
        }
        KeyCode::Enter => {
            app.handle_enter();
            false
        }
        KeyCode::Backspace => {
            if !app.is_root() {
                app.go_back();
            }
            false
        }
        // Panel switching and navigation
        KeyCode::Tab => {
            app.toggle_panel();
            false
        }
        KeyCode::Left | KeyCode::Char('h') => {
            if app.active_panel == Panel::Right {
                app.active_panel = Panel::Left;
            } else if !app.is_root() {
                app.go_back();
            }
            false
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if app.active_panel == Panel::Left {
                if let Some(item) = &app.selected_item {
                    if item.folder {
                        app.enter_folder();
                    } else {
                        app.active_panel = Panel::Right;
                    }
                } else {
                    app.active_panel = Panel::Right;
                }
            }
            false
        }

        // Actions - F2/N: New (context-sensitive)
        KeyCode::F(2) | KeyCode::Char('n') => {
            match app.active_panel {
                Panel::Left => app.open_new_item_dialog(false),
                Panel::Right => app.open_new_field_dialog(),
            }
            false
        }

        // F4/E: Edit
        KeyCode::F(4) | KeyCode::Char('e') => {
            match app.active_panel {
                Panel::Left => {
                    if app.selected_item.is_some() {
                        app.open_edit_item_dialog();
                    }
                }
                Panel::Right => {
                    if !app.fields.is_empty() {
                        app.open_edit_field_dialog();
                    }
                }
            }
            false
        }

        // F7/C: New folder
        KeyCode::F(7) | KeyCode::Char('c') => {
            if app.active_panel == Panel::Left {
                app.open_new_item_dialog(true);
            }
            false
        }

        // F8/D: Delete
        KeyCode::F(8) | KeyCode::Char('d') => {
            match app.active_panel {
                Panel::Left => {
                    if app.selected_item.is_some() {
                        app.mode = AppMode::Dialog(DialogType::DeleteItem);
                    }
                }
                Panel::Right => {
                    if !app.fields.is_empty() {
                        app.mode = AppMode::Dialog(DialogType::DeleteField);
                    }
                }
            }
            false
        }

        // P: Change password
        KeyCode::Char('p') => {
            app.open_change_password_dialog();
            false
        }

        _ => false,
    }
}

/// Handle keys in dialog mode
fn handle_dialog_key(app: &mut App, key: KeyCode, dialog_type: DialogType) -> bool {
    match key {
        KeyCode::Esc => {
            app.close_dialog();
            false
        }
        KeyCode::Enter => {
            match dialog_type {
                DialogType::NewItem | DialogType::NewFolder => {
                    app.create_item();
                }
                DialogType::NewField => {
                    app.create_field();
                }
                DialogType::EditItem => {
                    app.update_item();
                }
                DialogType::EditField => {
                    app.update_field();
                }
                DialogType::DeleteItem => {
                    app.delete_selected_item();
                }
                DialogType::DeleteField => {
                    app.delete_selected_field();
                }
                DialogType::FieldDetails => {
                    app.close_dialog();
                }
                DialogType::ChangePassword => {
                    app.try_change_password();
                }
            }
            false
        }
        KeyCode::Backspace => {
            if matches!(dialog_type, DialogType::ChangePassword) {
                match app.change_password_focus {
                    0 => { app.old_password.pop(); }
                    1 => { app.new_password.pop(); }
                    2 => { app.new_password_confirm.pop(); }
                    _ => {}
                }
            } else {
                app.input_buffer.pop();
            }
            false
        }
        KeyCode::Char(c) => {
            if matches!(dialog_type, DialogType::ChangePassword) {
                match app.change_password_focus {
                    0 => { app.old_password.push(c); }
                    1 => { app.new_password.push(c); }
                    2 => { app.new_password_confirm.push(c); }
                    _ => {}
                }
            } else {
                app.input_buffer.push(c);
            }
            false
        }
        KeyCode::Up => {
            if matches!(dialog_type, DialogType::NewField) {
                if app.dialog_selected > 0 {
                    app.dialog_selected -= 1;
                }
            } else if matches!(dialog_type, DialogType::ChangePassword) {
                if app.change_password_focus > 0 {
                    app.change_password_focus -= 1;
                }
            }
            false
        }
        KeyCode::Down | KeyCode::Tab => {
            if matches!(dialog_type, DialogType::NewField) {
                if app.dialog_selected + 1 < app.labels.len() {
                    app.dialog_selected += 1;
                }
            } else if matches!(dialog_type, DialogType::ChangePassword) {
                if app.change_password_focus < 2 {
                    app.change_password_focus += 1;
                } else {
                    app.change_password_focus = 0;
                }
            }
            false
        }
        _ => false,
    }
}
