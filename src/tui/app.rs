//! Application state management

use std::path::{Path, PathBuf};

use iwcore::{IWField, IWItem, IWLabel, ROOT_ID, Wallet};
use zeroize::{Zeroize, Zeroizing};

/// An empty buffer for a typed password. It is wiped on drop, and the
/// capacity is reserved up front so typing does not leave reallocated copies.
fn secret_buffer() -> Zeroizing<String> {
    Zeroizing::new(String::with_capacity(128))
}

/// Application mode
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    /// Password entry screen (unlock existing wallet)
    Password,
    /// Create new wallet screen
    CreateWallet,
    /// Main browsing mode
    Browse,
    /// Dialog is open
    Dialog(DialogType),
}

/// Type of dialog currently open
#[derive(Debug, Clone, PartialEq)]
pub enum DialogType {
    NewItem,
    NewFolder,
    NewField,
    EditItem,
    EditField,
    DeleteItem,
    DeleteField,
    FieldDetails,
    ChangePassword,
}

/// Which panel is active
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Panel {
    Left,
    Right,
}

/// Main application state
pub struct App {
    /// Current mode
    pub mode: AppMode,
    /// Wallet path
    pub wallet_path: Option<PathBuf>,
    /// Wallet instance (after unlock)
    pub wallet: Option<Wallet>,
    /// Active panel
    pub active_panel: Panel,

    // Left panel state
    /// Current folder being viewed
    pub current_folder_id: String,
    /// Current folder name (for display)
    pub current_folder_name: String,
    /// Items in current folder
    pub folder_items: Vec<IWItem>,
    /// Selected index in left panel (0 = ".." if not root)
    pub left_selected: usize,
    /// Parent folder ID stack for navigation
    pub folder_stack: Vec<(String, String)>, // (id, name)

    // Right panel state
    /// Currently selected item (if any)
    pub selected_item: Option<IWItem>,
    /// Fields of selected item
    pub fields: Vec<IWField>,
    /// Selected index in right panel
    pub right_selected: usize,

    // Dialog state
    /// Labels for field creation
    pub labels: Vec<IWLabel>,
    /// Input buffer for dialogs
    pub input_buffer: String,
    /// Secondary input (e.g., icon name)
    pub input_secondary: String,
    /// Dialog selection index
    pub dialog_selected: usize,
    /// Is creating folder (vs item)
    pub creating_folder: bool,

    // Status
    /// Status message
    pub message: Option<String>,
    /// Error message
    pub error: Option<String>,

    // Password entry
    pub password_input: Zeroizing<String>,
    pub password_confirm: Zeroizing<String>,
    pub password_error: Option<String>,
    /// Which field is focused in create wallet screen (0 = password, 1 = confirm)
    pub create_wallet_focus: usize,

    // Change password dialog
    pub old_password: Zeroizing<String>,
    pub new_password: Zeroizing<String>,
    pub new_password_confirm: Zeroizing<String>,
    /// Which field is focused (0 = old, 1 = new, 2 = confirm)
    pub change_password_focus: usize,
}

impl App {
    /// Create new application
    pub fn new(wallet_path: Option<&Path>) -> Self {
        // Determine mode based on whether wallet exists
        let mode = if let Some(path) = wallet_path {
            let db_path = path.join("nswallet.dat");
            if db_path.exists() {
                AppMode::Password
            } else {
                AppMode::CreateWallet
            }
        } else {
            AppMode::Browse // Will show about screen
        };

        Self {
            mode,
            wallet_path: wallet_path.map(|p| p.to_path_buf()),
            wallet: None,
            active_panel: Panel::Left,

            current_folder_id: ROOT_ID.to_string(),
            current_folder_name: "Root".to_string(),
            folder_items: Vec::new(),
            left_selected: 0,
            folder_stack: Vec::new(),

            selected_item: None,
            fields: Vec::new(),
            right_selected: 0,

            labels: Vec::new(),
            input_buffer: String::new(),
            input_secondary: String::new(),
            dialog_selected: 0,
            creating_folder: false,

            message: None,
            error: None,

            password_input: secret_buffer(),
            password_confirm: secret_buffer(),
            password_error: None,
            create_wallet_focus: 0,

            old_password: secret_buffer(),
            new_password: secret_buffer(),
            new_password_confirm: secret_buffer(),
            change_password_focus: 0,
        }
    }

    /// Open change password dialog
    pub fn open_change_password_dialog(&mut self) {
        self.wipe_change_password_fields();
        self.change_password_focus = 0;
        self.password_error = None;
        self.mode = AppMode::Dialog(DialogType::ChangePassword);
    }

    /// Try to change the wallet password
    pub fn try_change_password(&mut self) -> bool {
        // Validate new passwords match
        if self.new_password != self.new_password_confirm {
            self.password_error = Some("New passwords do not match".to_string());
            return false;
        }

        // Validate new password not empty
        if self.new_password.is_empty() {
            self.password_error = Some("New password cannot be empty".to_string());
            return false;
        }

        // Validate minimum length
        if self.new_password.len() < 4 {
            self.password_error = Some("Password must be at least 4 characters".to_string());
            return false;
        }

        if let Some(wallet) = &mut self.wallet {
            // Verify old password
            match wallet.check_password(&self.old_password) {
                Ok(true) => {}
                Ok(false) => {
                    self.password_error = Some("Current password is incorrect".to_string());
                    return false;
                }
                Err(e) => {
                    self.password_error = Some(format!("Error verifying password: {}", e));
                    return false;
                }
            }

            // Change password
            match wallet.change_password(&self.new_password) {
                Ok(_) => {
                    self.message = Some("Password changed successfully".to_string());
                    self.wipe_change_password_fields();
                    self.close_dialog();
                    true
                }
                Err(e) => {
                    self.password_error = Some(format!("Failed to change password: {}", e));
                    false
                }
            }
        } else {
            self.password_error = Some("Wallet not open".to_string());
            false
        }
    }

    /// Try to create a new wallet
    pub fn try_create_wallet(&mut self) -> bool {
        // Validate passwords match
        if self.password_input != self.password_confirm {
            self.password_error = Some("Passwords do not match".to_string());
            return false;
        }

        // Validate password not empty
        if self.password_input.is_empty() {
            self.password_error = Some("Password cannot be empty".to_string());
            return false;
        }

        // Validate minimum length
        if self.password_input.len() < 4 {
            self.password_error = Some("Password must be at least 4 characters".to_string());
            return false;
        }

        if let Some(path) = &self.wallet_path {
            match Wallet::create(path, &self.password_input, "en") {
                Ok(mut wallet) => {
                    // Load labels
                    if let Ok(labels) = wallet.get_labels() {
                        self.labels = labels;
                    }
                    self.wallet = Some(wallet);
                    self.wipe_entered_passwords();
                    self.mode = AppMode::Browse;
                    self.load_folder_items();
                    self.password_error = None;
                    self.message = Some("Wallet created successfully".to_string());
                    true
                }
                Err(e) => {
                    self.password_error = Some(format!("Failed to create wallet: {}", e));
                    false
                }
            }
        } else {
            false
        }
    }

    /// Try to unlock wallet with password
    pub fn try_unlock(&mut self) -> bool {
        if let Some(path) = &self.wallet_path {
            match Wallet::open(path) {
                Ok(mut wallet) => {
                    match wallet.unlock(&self.password_input) {
                        Ok(true) => {
                            // Load labels
                            if let Ok(labels) = wallet.get_labels() {
                                self.labels = labels;
                            }
                            self.message = unlock_notice(&wallet);
                            self.wallet = Some(wallet);
                            self.wipe_entered_passwords();
                            self.mode = AppMode::Browse;
                            self.load_folder_items();
                            self.password_error = None;
                            true
                        }
                        Ok(false) => {
                            self.password_error = Some("Invalid password".to_string());
                            self.password_input.zeroize();
                            false
                        }
                        Err(e) => {
                            self.password_error = Some(format!("Error: {}", e));
                            self.password_input.zeroize();
                            false
                        }
                    }
                }
                Err(e) => {
                    self.password_error = Some(format!("Cannot open wallet: {}", e));
                    self.password_input.zeroize();
                    false
                }
            }
        } else {
            false
        }
    }

    /// Load items for current folder
    pub fn load_folder_items(&mut self) {
        let folder_id = self.current_folder_id.clone();
        if let Some(wallet) = &mut self.wallet {
            match wallet.get_items_by_parent(&folder_id) {
                Ok(items) => {
                    self.folder_items = items;
                    self.left_selected = 0;
                    // Auto-select first item if any
                    self.update_selected_item();
                }
                Err(e) => {
                    self.error = Some(format!("Failed to load items: {}", e));
                }
            }
        }
    }

    /// Update the selected item and load its fields
    pub fn update_selected_item(&mut self) {
        let is_root = self.current_folder_id == ROOT_ID;
        let item_index = if is_root {
            self.left_selected
        } else {
            // Account for ".." entry
            if self.left_selected == 0 {
                self.selected_item = None;
                self.fields.clear();
                self.right_selected = 0;
                return;
            }
            self.left_selected - 1
        };

        if let Some(item) = self.folder_items.get(item_index).cloned() {
            let is_folder = item.folder;
            let item_id = item.item_id.clone();
            self.selected_item = Some(item);
            if !is_folder {
                self.load_fields(&item_id);
            } else {
                self.fields.clear();
                self.right_selected = 0;
            }
        } else {
            self.selected_item = None;
            self.fields.clear();
            self.right_selected = 0;
        }
    }

    /// Load fields for an item
    fn load_fields(&mut self, item_id: &str) {
        let item_id = item_id.to_string();
        if let Some(wallet) = &mut self.wallet {
            match wallet.get_fields_by_item(&item_id) {
                Ok(fields) => {
                    self.fields = fields;
                    self.right_selected = 0;
                }
                Err(e) => {
                    self.error = Some(format!("Failed to load fields: {}", e));
                }
            }
        }
    }

    /// Navigate into a folder
    pub fn enter_folder(&mut self) {
        if let Some(item) = &self.selected_item
            && item.folder
        {
            // Push current folder to stack
            self.folder_stack.push((
                self.current_folder_id.clone(),
                self.current_folder_name.clone(),
            ));
            // Enter new folder
            self.current_folder_id = item.item_id.clone();
            self.current_folder_name = item.name.clone();
            self.load_folder_items();
        }
    }

    /// Navigate back to parent folder
    pub fn go_back(&mut self) {
        // Remember which folder we're leaving
        let leaving_folder_id = self.current_folder_id.clone();

        if let Some((parent_id, parent_name)) = self.folder_stack.pop() {
            self.current_folder_id = parent_id;
            self.current_folder_name = parent_name;
            self.load_folder_items();

            // Select the folder we just left
            self.select_item_by_id(&leaving_folder_id);
        }
    }

    /// Select an item by ID in the left panel
    fn select_item_by_id(&mut self, id: &str) {
        let offset = if self.is_root() { 0 } else { 1 }; // Account for ".."
        for (i, item) in self.folder_items.iter().enumerate() {
            if item.item_id == id {
                self.left_selected = i + offset;
                self.update_selected_item();
                return;
            }
        }
    }

    /// Check if we're in the root folder
    pub fn is_root(&self) -> bool {
        self.current_folder_id == ROOT_ID
    }

    /// Get the current path for display
    pub fn current_path(&self) -> String {
        if self.folder_stack.is_empty() {
            "/".to_string()
        } else {
            let mut path = String::from("/");
            // Skip the first entry (Root) in folder_stack
            for (_, name) in self.folder_stack.iter().skip(1) {
                path.push_str(name);
                path.push('/');
            }
            path.push_str(&self.current_folder_name);
            path.push('/');
            path
        }
    }

    /// Move selection up in current panel
    pub fn move_up(&mut self) {
        match self.active_panel {
            Panel::Left => {
                if self.left_selected > 0 {
                    self.left_selected -= 1;
                    self.update_selected_item();
                }
            }
            Panel::Right => {
                if self.right_selected > 0 {
                    self.right_selected -= 1;
                }
            }
        }
    }

    /// Move selection down in current panel
    pub fn move_down(&mut self) {
        match self.active_panel {
            Panel::Left => {
                let max = if self.is_root() {
                    self.folder_items.len()
                } else {
                    self.folder_items.len() + 1 // +1 for ".."
                };
                if self.left_selected + 1 < max {
                    self.left_selected += 1;
                    self.update_selected_item();
                }
            }
            Panel::Right => {
                if self.right_selected + 1 < self.fields.len() {
                    self.right_selected += 1;
                }
            }
        }
    }

    /// Handle Enter key
    pub fn handle_enter(&mut self) {
        match self.active_panel {
            Panel::Left => {
                if !self.is_root() && self.left_selected == 0 {
                    // Selected ".." - go back
                    self.go_back();
                } else if let Some(item) = &self.selected_item
                    && item.folder
                {
                    self.enter_folder();
                }
                // For regular items, fields are already shown
            }
            Panel::Right => {
                // Show field details
                if !self.fields.is_empty() {
                    self.mode = AppMode::Dialog(DialogType::FieldDetails);
                }
            }
        }
    }

    /// Toggle active panel
    pub fn toggle_panel(&mut self) {
        self.active_panel = match self.active_panel {
            Panel::Left => Panel::Right,
            Panel::Right => Panel::Left,
        };
    }

    /// Open new item dialog
    pub fn open_new_item_dialog(&mut self, is_folder: bool) {
        self.creating_folder = is_folder;
        self.input_buffer.clear();
        self.input_secondary = "folder".to_string(); // Default icon
        self.mode = AppMode::Dialog(if is_folder {
            DialogType::NewFolder
        } else {
            DialogType::NewItem
        });
    }

    /// Open new field dialog
    pub fn open_new_field_dialog(&mut self) {
        if self.selected_item.is_some() && !self.selected_item.as_ref().unwrap().folder {
            self.input_buffer.clear();
            self.dialog_selected = 0;
            self.mode = AppMode::Dialog(DialogType::NewField);
        }
    }

    /// Create a new item
    pub fn create_item(&mut self) {
        let created_name = self.input_buffer.trim().to_string();
        if created_name.is_empty() {
            self.error = Some("Name cannot be empty".to_string());
            self.mode = AppMode::Browse;
            return;
        }

        if let Some(wallet) = &mut self.wallet {
            let icon = if self.input_secondary.is_empty() {
                "folder"
            } else {
                &self.input_secondary
            };

            match wallet.add_item(
                &created_name,
                icon,
                self.creating_folder,
                Some(&self.current_folder_id),
            ) {
                Ok(_) => {
                    self.message = Some(format!(
                        "Created {}",
                        if self.creating_folder {
                            "folder"
                        } else {
                            "item"
                        }
                    ));
                    self.load_folder_items();
                    // Find and select the newly created item
                    self.select_item_by_name(&created_name);
                }
                Err(e) => {
                    self.error = Some(format!("Failed to create: {}", e));
                }
            }
        }
        self.mode = AppMode::Browse;
    }

    /// Select an item by name in the left panel
    fn select_item_by_name(&mut self, name: &str) {
        let offset = if self.is_root() { 0 } else { 1 }; // Account for ".."
        for (i, item) in self.folder_items.iter().enumerate() {
            if item.name == name {
                self.left_selected = i + offset;
                self.update_selected_item();
                return;
            }
        }
    }

    /// Create a new field
    pub fn create_field(&mut self) {
        let item_id = self.selected_item.as_ref().map(|i| i.item_id.clone());
        let field_type = self
            .labels
            .get(self.dialog_selected)
            .map(|l| l.field_type.clone());
        let value = Zeroizing::new(self.input_buffer.trim().to_string());

        if let (Some(item_id), Some(field_type)) = (item_id, field_type) {
            if let Some(wallet) = &mut self.wallet {
                match wallet.add_field(&item_id, &field_type, &value, None) {
                    Ok(_) => {
                        self.message = Some("Field added".to_string());
                        self.load_fields(&item_id);
                    }
                    Err(e) => {
                        self.error = Some(format!("Failed to add field: {}", e));
                    }
                }
            }
        } else {
            self.error = Some("Invalid field type".to_string());
        }
        self.input_buffer.zeroize();
        self.mode = AppMode::Browse;
    }

    /// Delete selected item
    pub fn delete_selected_item(&mut self) {
        if let (Some(wallet), Some(item)) = (&mut self.wallet, &self.selected_item.clone()) {
            match wallet.delete_item(&item.item_id) {
                Ok(_) => {
                    self.message = Some("Item deleted".to_string());
                    self.load_folder_items();
                }
                Err(e) => {
                    self.error = Some(format!("Failed to delete: {}", e));
                }
            }
        }
        self.mode = AppMode::Browse;
    }

    /// Delete selected field
    pub fn delete_selected_field(&mut self) {
        let item_id = self.selected_item.as_ref().map(|i| i.item_id.clone());
        let field_id = self
            .fields
            .get(self.right_selected)
            .map(|f| f.field_id.clone());

        if let (Some(item_id), Some(field_id)) = (item_id, field_id)
            && let Some(wallet) = &mut self.wallet
        {
            match wallet.delete_field(&item_id, &field_id) {
                Ok(_) => {
                    self.message = Some("Field deleted".to_string());
                    self.load_fields(&item_id);
                }
                Err(e) => {
                    self.error = Some(format!("Failed to delete: {}", e));
                }
            }
        }
        self.mode = AppMode::Browse;
    }

    /// Close dialog
    pub fn close_dialog(&mut self) {
        self.mode = AppMode::Browse;
        // The buffer may hold a field value (a password) being edited.
        self.input_buffer.zeroize();
        self.input_secondary.clear();
    }

    /// Wipe the passwords typed on the unlock / create wallet screen.
    fn wipe_entered_passwords(&mut self) {
        self.password_input.zeroize();
        self.password_confirm.zeroize();
    }

    /// Wipe the passwords typed in the change password dialog.
    pub fn wipe_change_password_fields(&mut self) {
        self.old_password.zeroize();
        self.new_password.zeroize();
        self.new_password_confirm.zeroize();
    }

    /// Open edit item dialog
    pub fn open_edit_item_dialog(&mut self) {
        if let Some(item) = &self.selected_item {
            self.input_buffer = item.name.clone();
            self.mode = AppMode::Dialog(DialogType::EditItem);
        }
    }

    /// Open edit field dialog
    pub fn open_edit_field_dialog(&mut self) {
        if let Some(field) = self.fields.get(self.right_selected) {
            self.input_buffer = field.value.clone();
            self.mode = AppMode::Dialog(DialogType::EditField);
        }
    }

    /// Update item name
    pub fn update_item(&mut self) {
        let new_name = self.input_buffer.trim().to_string();
        if new_name.is_empty() {
            self.error = Some("Name cannot be empty".to_string());
            self.mode = AppMode::Browse;
            return;
        }

        let item_id = self.selected_item.as_ref().map(|i| i.item_id.clone());
        if let Some(item_id) = item_id
            && let Some(wallet) = &mut self.wallet
        {
            match wallet.update_item_name(&item_id, &new_name) {
                Ok(_) => {
                    self.message = Some("Item updated".to_string());
                    self.load_folder_items();
                    self.select_item_by_name(&new_name);
                }
                Err(e) => {
                    self.error = Some(format!("Failed to update: {}", e));
                }
            }
        }
        self.mode = AppMode::Browse;
    }

    /// Update field value
    pub fn update_field(&mut self) {
        let new_value = Zeroizing::new(self.input_buffer.trim().to_string());
        let item_id = self.selected_item.as_ref().map(|i| i.item_id.clone());
        let field_id = self
            .fields
            .get(self.right_selected)
            .map(|f| f.field_id.clone());

        if let (Some(item_id), Some(field_id)) = (item_id, field_id)
            && let Some(wallet) = &mut self.wallet
        {
            match wallet.update_field(&field_id, &new_value, None) {
                Ok(_) => {
                    self.message = Some("Field updated".to_string());
                    self.load_fields(&item_id);
                }
                Err(e) => {
                    self.error = Some(format!("Failed to update: {}", e));
                }
            }
        }
        self.input_buffer.zeroize();
        self.mode = AppMode::Browse;
    }
}

/// Message shown after a successful unlock when iwcore did more than unlock:
/// the one-time upgrade of an old (pre-v6) vault, records it could not
/// decrypt, or an old-format snapshot still in the wallet folder.
fn unlock_notice(wallet: &Wallet) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(summary) = wallet.last_migration_summary() {
        parts.push(format!(
            "Vault upgraded to the current format ({} items, {} fields)",
            summary.items_migrated, summary.fields_migrated
        ));
    }
    match wallet.quarantine_count() {
        Ok(0) | Err(_) => {}
        Ok(n) => parts.push(format!(
            "{} old records could not be decrypted and are kept aside",
            n
        )),
    }
    if wallet.has_pre_v6_snapshot() {
        parts.push(format!(
            "old-format copy {} is still in the wallet folder; delete it once you have a current backup",
            iwcore::business::wallet::PRE_V6_BACKUP_FILENAME
        ));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("; "))
    }
}
