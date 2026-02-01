//! IntelliWallet CLI
//!
//! Command-line interface for the IntelliWallet password manager.

mod tui;

use clap::{Parser, Subcommand};
use iwcore::{BackupManager, BackupType, DATABASE_FILENAME, DB_VERSION};
use iwcore::backup::is_backup_compatible;
use std::path::Path;

/// IntelliWallet CLI - Password Manager
#[derive(Parser)]
#[command(name = "iwcli")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the wallet folder
    #[arg(short, long)]
    wallet: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Unlock the wallet
    Unlock,
    /// Lock the wallet
    Lock,
    /// List items in the current folder
    List {
        /// Parent ID to list (optional)
        parent_id: Option<String>,
    },
    /// Show item details
    Show {
        /// Item ID to show
        item_id: String,
    },
    /// Search items and fields
    Search {
        /// Search query
        query: String,
    },
    /// Show version info
    Version,
    /// Create a backup of the wallet database
    Backup {
        /// Path to the backup folder
        #[arg(long)]
        backup_folder: String,
    },
    /// Restore wallet from a backup file
    Restore {
        /// Path to the backup ZIP file
        #[arg(long)]
        backup: String,
        /// Password to verify after restore (optional)
        #[arg(long)]
        password: Option<String>,
    },
    /// List available backups in a folder
    Backups {
        /// Path to the backup folder
        #[arg(long)]
        backup_folder: String,
    },
}

fn cmd_backup(wallet_path: &str, backup_folder: &str) {
    let wallet_dir = Path::new(wallet_path);
    let db_path = wallet_dir.join(DATABASE_FILENAME);

    if !db_path.exists() {
        eprintln!("Error: wallet database not found at {}", db_path.display());
        std::process::exit(1);
    }

    let backup_dir = Path::new(backup_folder);
    if !backup_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(backup_dir) {
            eprintln!("Error: failed to create backup folder: {}", e);
            std::process::exit(1);
        }
    }

    let manager = BackupManager::new(backup_dir);
    match manager.create_backup_from_path(&db_path, true) {
        Ok(path) => println!("{}", path.display()),
        Err(e) => {
            eprintln!("Error: backup failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_restore(wallet_path: &str, backup_path: &str, password: Option<&str>) {
    let backup_file = Path::new(backup_path);
    if !backup_file.exists() {
        eprintln!("Error: backup file not found: {}", backup_path);
        std::process::exit(1);
    }

    let wallet_dir = Path::new(wallet_path);
    let db_path = wallet_dir.join(DATABASE_FILENAME);

    let manager = BackupManager::new(wallet_dir);

    // Verify backup integrity
    match manager.verify_backup(backup_file) {
        Ok(true) => {}
        Ok(false) => {
            eprintln!("Error: backup file is invalid or empty");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: backup verification failed: {}", e);
            std::process::exit(1);
        }
    }

    // Check version compatibility
    match is_backup_compatible(backup_file, DB_VERSION) {
        Ok(true) => {}
        Ok(false) => {
            eprintln!("Error: backup database version is not compatible with this version of iwcli");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: version check failed: {}", e);
            std::process::exit(1);
        }
    }

    // If password provided, verify it by extracting to temp and opening
    if let Some(pwd) = password {
        let temp_dir = match tempfile::TempDir::new() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Error: failed to create temp directory: {}", e);
                std::process::exit(1);
            }
        };

        match manager.extract_backup(backup_file, temp_dir.path()) {
            Ok(_) => {
                match iwcore::Wallet::open(temp_dir.path()) {
                    Ok(mut wallet) => {
                        match wallet.unlock(pwd) {
                            Ok(true) => {}
                            Ok(false) => {
                                eprintln!("Error: incorrect password");
                                std::process::exit(1);
                            }
                            Err(e) => {
                                eprintln!("Error: password verification failed: {}", e);
                                std::process::exit(1);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: failed to open backup database: {}", e);
                        std::process::exit(1);
                    }
                }
                // temp_dir dropped automatically
            }
            Err(e) => {
                eprintln!("Error: failed to extract backup for verification: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Ensure wallet directory exists
    if !wallet_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(wallet_dir) {
            eprintln!("Error: failed to create wallet folder: {}", e);
            std::process::exit(1);
        }
    }

    // Restore
    match manager.restore_backup(backup_file, &db_path) {
        Ok(()) => println!("Restored to {}", db_path.display()),
        Err(e) => {
            eprintln!("Error: restore failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_backups(backup_folder: &str) {
    let backup_dir = Path::new(backup_folder);
    let manager = BackupManager::new(backup_dir);

    let backups = match manager.list_backups() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error: failed to list backups: {}", e);
            std::process::exit(1);
        }
    };

    if backups.is_empty() {
        println!("No backups found in {}", backup_folder);
        return;
    }

    println!("{:<22} {:<10} {:>10}  {}", "Date", "Type", "Size", "Filename");
    println!("{}", "-".repeat(70));

    for info in &backups {
        let date_str = info.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
        let type_str = match info.backup_type {
            BackupType::Auto => "auto",
            BackupType::Manual => "manual",
            BackupType::Imported => "imported",
        };
        let size_str = format_size(info.size);
        let filename = info.path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?");
        println!("{:<22} {:<10} {:>10}  {}", date_str, type_str, size_str, filename);
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Version) => {
            println!("iwcli version {}", env!("CARGO_PKG_VERSION"));
            println!("iwcore version {}", iwcore::DB_VERSION);
        }
        Some(Commands::Backup { backup_folder }) => {
            let wallet = cli.wallet.as_deref().unwrap_or_else(|| {
                eprintln!("Error: --wallet is required for backup");
                std::process::exit(1);
            });
            cmd_backup(wallet, backup_folder);
        }
        Some(Commands::Restore { backup, password }) => {
            let wallet = cli.wallet.as_deref().unwrap_or_else(|| {
                eprintln!("Error: --wallet is required for restore");
                std::process::exit(1);
            });
            cmd_restore(wallet, backup, password.as_deref());
        }
        Some(Commands::Backups { backup_folder }) => {
            cmd_backups(backup_folder);
        }
        Some(_) => {
            println!("Command not yet implemented");
        }
        None => {
            // Run TUI mode (default)
            if cli.wallet.is_none() {
                // Show about screen if no wallet specified
                if let Err(e) = tui::show_about() {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            } else {
                // Run full TUI with wallet
                if let Err(e) = tui::run(cli.wallet.as_deref()) {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
