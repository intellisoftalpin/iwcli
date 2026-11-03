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

#[cfg(test)]
mod tests {
    use super::*;
    use iwcore::Wallet;
    use std::io::Write;
    use tempfile::TempDir;

    const PASSWORD: &str = "correct horse";

    /// Create a wallet through iwcore with one item and one field, closed again.
    fn make_wallet(dir: &Path) {
        let mut wallet = Wallet::create(dir, PASSWORD, "en").unwrap();
        let item = wallet
            .add_item("Mail", "mail", false, Some(iwcore::ROOT_ID))
            .unwrap();
        let label = wallet.get_labels().unwrap()[0].field_type.clone();
        wallet.add_field(&item, &label, "s3cret", None).unwrap();
        wallet.close();
    }

    /// Write a ZIP holding `nswallet.dat` with the given bytes.
    fn write_zip(path: &Path, db_bytes: &[u8]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file(DATABASE_FILENAME, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(db_bytes).unwrap();
        zip.finish().unwrap();
    }

    fn only_backup(folder: &Path) -> PathBuf {
        let list = BackupManager::new(folder).list_backups().unwrap();
        assert_eq!(list.len(), 1);
        list[0].path.clone()
    }

    #[test]
    fn backup_writes_a_manual_backup_and_creates_the_folder() {
        let tmp = TempDir::new().unwrap();
        let wallet = tmp.path().join("wallet");
        let folder = tmp.path().join("nested").join("backups");
        make_wallet(&wallet);

        let path = backup(&wallet, &folder).unwrap();

        assert!(path.is_file());
        assert_eq!(path.parent().unwrap(), folder);
        let list = BackupManager::new(&folder).list_backups().unwrap();
        assert_eq!(list.len(), 1);
        assert!(matches!(list[0].backup_type, BackupType::Manual));
    }

    #[test]
    fn backup_without_wallet_database_fails() {
        let tmp = TempDir::new().unwrap();
        let err = backup(tmp.path(), &tmp.path().join("b")).unwrap_err();
        assert!(err.starts_with("wallet database not found"), "{err}");
        assert!(!tmp.path().join("b").exists());
    }

    #[test]
    fn backups_lists_header_and_one_line_per_backup() {
        let tmp = TempDir::new().unwrap();
        let wallet = tmp.path().join("wallet");
        let folder = tmp.path().join("backups");
        make_wallet(&wallet);
        let path = backup(&wallet, &folder).unwrap();

        let lines = backups(&folder).unwrap();

        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("Date"));
        assert!(lines[0].ends_with("Filename"));
        assert_eq!(lines[1], "-".repeat(70));
        let name = path.file_name().unwrap().to_str().unwrap();
        assert!(lines[2].contains("manual"), "{}", lines[2]);
        assert!(lines[2].ends_with(name), "{}", lines[2]);
    }

    #[test]
    fn backups_in_empty_folder_says_none_found() {
        let tmp = TempDir::new().unwrap();
        let lines = backups(tmp.path()).unwrap();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].starts_with("No backups found in "));
    }

    #[test]
    fn restore_with_correct_password_gives_a_working_wallet() {
        let tmp = TempDir::new().unwrap();
        let wallet = tmp.path().join("wallet");
        let folder = tmp.path().join("backups");
        make_wallet(&wallet);
        let file = backup(&wallet, &folder).unwrap();
        let target = tmp.path().join("restored");

        let outcome = restore(&target, &file, Some(PASSWORD)).unwrap();

        assert_eq!(outcome.db_path, target.join(DATABASE_FILENAME));
        assert_eq!(outcome.inspection.db_version, DB_VERSION);
        assert_eq!(outcome.inspection.item_count, 1);
        assert!(!outcome.needs_upgrade());
        let mut restored = Wallet::open(&target).unwrap();
        assert!(restored.unlock(PASSWORD).unwrap());
        let items = restored.get_items_by_parent(iwcore::ROOT_ID).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "Mail");
    }

    #[test]
    fn restore_without_password_replaces_the_database() {
        let tmp = TempDir::new().unwrap();
        let wallet = tmp.path().join("wallet");
        make_wallet(&wallet);
        let file = backup(&wallet, &tmp.path().join("b")).unwrap();
        // An existing wallet in the target is replaced.
        let target = tmp.path().join("other");
        Wallet::create(&target, "other password", "en")
            .unwrap()
            .close();

        restore(&target, &file, None).unwrap();

        let mut restored = Wallet::open(&target).unwrap();
        assert!(restored.unlock(PASSWORD).unwrap());
    }

    #[test]
    fn restore_with_wrong_password_leaves_the_target_untouched() {
        let tmp = TempDir::new().unwrap();
        let wallet = tmp.path().join("wallet");
        make_wallet(&wallet);
        backup(&wallet, &tmp.path().join("b")).unwrap();
        let file = only_backup(&tmp.path().join("b"));
        let target = tmp.path().join("restored");

        let err = restore(&target, &file, Some("wrong")).unwrap_err();

        assert_eq!(err, "incorrect password");
        assert!(!target.exists());
    }

    #[test]
    fn restore_of_missing_file_fails() {
        let tmp = TempDir::new().unwrap();
        let err = restore(tmp.path(), &tmp.path().join("nope.zip"), None).unwrap_err();
        assert!(err.starts_with("backup file not found"), "{err}");
    }

    #[test]
    fn restore_refuses_a_file_that_is_not_a_zip() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("fake.zip");
        std::fs::write(&file, b"definitely not a zip").unwrap();
        let err = restore(&tmp.path().join("w"), &file, None).unwrap_err();
        assert!(err.starts_with("backup verification failed"), "{err}");
        assert!(!tmp.path().join("w").exists());
    }

    #[test]
    fn restore_refuses_a_zip_with_an_empty_database() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("empty.zip");
        write_zip(&file, b"");
        let err = restore(&tmp.path().join("w"), &file, None).unwrap_err();
        assert_eq!(err, "backup file is invalid or empty");
    }

    #[test]
    fn restore_refuses_a_database_that_is_not_sqlite() {
        let tmp = TempDir::new().unwrap();
        let wallet = tmp.path().join("wallet");
        make_wallet(&wallet);
        let before = std::fs::read(wallet.join(DATABASE_FILENAME)).unwrap();
        let file = tmp.path().join("bad.zip");
        write_zip(&file, &[0x42; 4096]);

        let err = restore(&wallet, &file, None).unwrap_err();

        assert!(err.starts_with("backup refused"), "{err}");
        assert_eq!(
            std::fs::read(wallet.join(DATABASE_FILENAME)).unwrap(),
            before
        );
    }

    #[test]
    fn restore_refuses_an_sqlite_file_without_wallet_tables() {
        let tmp = TempDir::new().unwrap();
        // A valid SQLite header followed by garbage pages.
        let mut db = b"SQLite format 3\0".to_vec();
        db.resize(4096, 0xAB);
        let file = tmp.path().join("crafted.zip");
        write_zip(&file, &db);
        let target = tmp.path().join("w");

        let err = restore(&target, &file, Some(PASSWORD)).unwrap_err();

        assert!(err.starts_with("backup refused"), "{err}");
        assert!(!target.exists());
    }

    #[test]
    fn version_compatibility() {
        assert!(is_version_compatible(DB_VERSION));
        assert!(is_version_compatible("5"));
        assert!(is_version_compatible("garbage")); // counts as 1, like iwcore
        assert!(!is_version_compatible("7"));
        assert!(!is_version_compatible("99"));
    }

    #[test]
    fn backup_type_names() {
        assert_eq!(backup_type_name(&BackupType::Auto), "auto");
        assert_eq!(backup_type_name(&BackupType::Manual), "manual");
        assert_eq!(backup_type_name(&BackupType::Imported), "imported");
    }

    #[test]
    fn sizes_are_formatted_in_b_kb_mb() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(5 * 1024 * 1024 + 512 * 1024), "5.5 MB");
    }
}
