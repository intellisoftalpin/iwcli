//! Non-interactive commands: backup, restore, backups.
//!
//! Every function returns `Result<_, String>` with the message to show; the
//! caller in `main` prints it and sets the exit code.

use std::path::{Path, PathBuf};

use iwcore::backup::{BackupInspection, check_backup_password_in, inspect_backup_in};
use iwcore::{BackupManager, BackupType, DATABASE_FILENAME, DB_VERSION};

/// First database version written with the v6 scheme (Argon2id + XChaCha20-Poly1305).
const V6: u32 = 6;

/// Outcome of a successful restore.
#[derive(Debug)]
pub struct RestoreOutcome {
    /// Path of the restored database.
    pub db_path: PathBuf,
    /// What the backup held, as read before the restore.
    pub inspection: BackupInspection,
}

impl RestoreOutcome {
    /// True when the restored database is older than v6. iwcore upgrades it
    /// on the next unlock and keeps a pre-v6 snapshot in the wallet folder.
    pub fn needs_upgrade(&self) -> bool {
        db_version_number(&self.inspection.db_version) < V6
    }
}

/// Create a manual backup of the wallet database in `backup_folder`.
/// Returns the path of the new backup file.
pub fn backup(wallet_path: &Path, backup_folder: &Path) -> Result<PathBuf, String> {
    let db_path = wallet_path.join(DATABASE_FILENAME);
    if !db_path.exists() {
        return Err(format!(
            "wallet database not found at {}",
            db_path.display()
        ));
    }

    if !backup_folder.exists() {
        std::fs::create_dir_all(backup_folder)
            .map_err(|e| format!("failed to create backup folder: {}", e))?;
    }

    BackupManager::new(backup_folder)
        .create_backup_from_path(&db_path, true)
        .map_err(|e| format!("backup failed: {}", e))
}

/// Restore the wallet in `wallet_path` from `backup_file`.
///
/// The backup is untrusted input. Before anything in the wallet folder is
/// touched, iwcore checks it (SQLite header, integrity check, real wallet
/// tables, size and compression limits) through a read-only connection on a
/// temporary copy, and its version must not be newer than this build's.
/// An optional password is verified on that copy too; nothing is migrated or
/// written there. The restore itself replaces the database atomically.
pub fn restore(
    wallet_path: &Path,
    backup_file: &Path,
    password: Option<&str>,
) -> Result<RestoreOutcome, String> {
    if !backup_file.exists() {
        return Err(format!("backup file not found: {}", backup_file.display()));
    }

    let manager = BackupManager::new(wallet_path);
    match manager.verify_backup(backup_file) {
        Ok(true) => {}
        Ok(false) => return Err("backup file is invalid or empty".to_string()),
        Err(e) => return Err(format!("backup verification failed: {}", e)),
    }

    let temp_dir =
        tempfile::TempDir::new().map_err(|e| format!("failed to create temp directory: {}", e))?;

    let inspection = inspect_backup_in(backup_file, temp_dir.path())
        .map_err(|e| format!("backup refused: {}", e))?;

    if !is_version_compatible(&inspection.db_version) {
        return Err(
            "backup database version is not compatible with this version of iwcli".to_string(),
        );
    }

    if let Some(pwd) = password {
        match check_backup_password_in(backup_file, pwd, temp_dir.path()) {
            Ok(true) => {}
            Ok(false) => return Err("incorrect password".to_string()),
            Err(e) => return Err(format!("password verification failed: {}", e)),
        }
    }
    drop(temp_dir);

    if !wallet_path.exists() {
        std::fs::create_dir_all(wallet_path)
            .map_err(|e| format!("failed to create wallet folder: {}", e))?;
    }

    let db_path = wallet_path.join(DATABASE_FILENAME);
    manager
        .restore_backup(backup_file, &db_path)
        .map_err(|e| format!("restore failed: {}", e))?;

    Ok(RestoreOutcome {
        db_path,
        inspection,
    })
}

/// List the backups in `backup_folder` as printable lines (header first).
/// An empty folder gives a single "No backups found" line.
pub fn backups(backup_folder: &Path) -> Result<Vec<String>, String> {
    let list = BackupManager::new(backup_folder)
        .list_backups()
        .map_err(|e| format!("failed to list backups: {}", e))?;

    if list.is_empty() {
        return Ok(vec![format!(
            "No backups found in {}",
            backup_folder.display()
        )]);
    }

    let mut lines = vec![
        format!("{:<22} {:<10} {:>10}  Filename", "Date", "Type", "Size"),
        "-".repeat(70),
    ];
    for info in &list {
        let date_str = info.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
        let type_str = backup_type_name(&info.backup_type);
        let size_str = format_size(info.size);
        let filename = info
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?");
        lines.push(format!(
            "{:<22} {:<10} {:>10}  {}",
            date_str, type_str, size_str, filename
        ));
    }
    Ok(lines)
}

/// Short name of a backup type for listings.
pub fn backup_type_name(backup_type: &BackupType) -> &'static str {
    match backup_type {
        BackupType::Auto => "auto",
        BackupType::Manual => "manual",
        BackupType::Imported => "imported",
    }
}

/// Human-readable file size.
pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Database version as a number; unparsable versions count as 1, like iwcore.
fn db_version_number(version: &str) -> u32 {
    version.trim().parse().unwrap_or(1)
}

/// A backup is compatible when its database version is not newer than ours.
fn is_version_compatible(backup_version: &str) -> bool {
    db_version_number(backup_version) <= db_version_number(DB_VERSION)
}
