//! IntelliWallet CLI
//!
//! Command-line interface for the IntelliWallet password manager.

mod commands;
mod totp;
mod tui;

use clap::{Parser, Subcommand};
use std::path::Path;
use zeroize::Zeroizing;

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

/// Print an error and exit with status 1.
fn fail(message: &str) -> ! {
    eprintln!("Error: {}", message);
    std::process::exit(1);
}

/// The `--wallet` path, required by `command`.
fn require_wallet<'a>(wallet: Option<&'a str>, command: &str) -> &'a Path {
    match wallet {
        Some(w) => Path::new(w),
        None => fail(&format!("--wallet is required for {}", command)),
    }
}

fn main() {
    let mut cli = Cli::parse();

    match &mut cli.command {
        Some(Commands::Version) => {
            println!("iwcli version {}", env!("CARGO_PKG_VERSION"));
            println!("iwcore version {}", iwcore::DB_VERSION);
        }
        Some(Commands::Backup { backup_folder }) => {
            let wallet = require_wallet(cli.wallet.as_deref(), "backup");
            match commands::backup(wallet, Path::new(backup_folder)) {
                Ok(path) => println!("{}", path.display()),
                Err(e) => fail(&e),
            }
        }
        Some(Commands::Restore { backup, password }) => {
            // Keep the password only in a buffer that is wiped on drop.
            let password = password.take().map(Zeroizing::new);
            let wallet = require_wallet(cli.wallet.as_deref(), "restore");
            match commands::restore(wallet, Path::new(backup), password.as_deref().map(|p| p.as_str())) {
                Ok(outcome) => {
                    println!("Restored to {}", outcome.db_path.display());
                    if outcome.needs_upgrade() {
                        println!(
                            "Note: the backup uses the old (pre-v6) vault format; it is upgraded on the next unlock."
                        );
                    }
                }
                Err(e) => fail(&e),
            }
        }
        Some(Commands::Backups { backup_folder }) => match commands::backups(Path::new(backup_folder)) {
            Ok(lines) => {
                for line in lines {
                    println!("{}", line);
                }
            }
            Err(e) => fail(&e),
        },
        Some(_) => {
            println!("Command not yet implemented");
        }
        None => {
            // Run TUI mode (default)
            if cli.wallet.is_none() {
                // Show about screen if no wallet specified
                if let Err(e) = tui::show_about() {
                    fail(&e.to_string());
                }
            } else if let Err(e) = tui::run(cli.wallet.as_deref()) {
                fail(&e.to_string());
            }
        }
    }
}
