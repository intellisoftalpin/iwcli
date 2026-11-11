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
        /// Password to check against the backup before restoring. Without it
        /// iwcli asks for it on the terminal (input hidden; Enter skips the
        /// check). Passing it here leaves it in shell history and visible to
        /// other users' `ps`: for scripts only.
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

/// Ask for the backup password with echo off, when there is a terminal to ask
/// on. An empty answer, or no terminal (a script), skips the password check.
fn ask_backup_password() -> Option<Zeroizing<String>> {
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() {
        return None;
    }
    match rpassword::prompt_password("Backup password (Enter to skip the check): ") {
        Ok(typed) => {
            let typed = Zeroizing::new(typed);
            (!typed.is_empty()).then_some(typed)
        }
        Err(e) => fail(&format!("could not read the password: {}", e)),
    }
}

fn main() {
    let mut cli = Cli::parse();

    match &mut cli.command {
        Some(Commands::Version) => {
            println!("iwcli version {}", env!("CARGO_PKG_VERSION"));
            println!("wallet database format {}", iwcore::DB_VERSION);
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
            let password = match password.take() {
                Some(given) => {
                    eprintln!(
                        "Warning: --password leaves the password in shell history and visible to other users; omit it to be asked instead."
                    );
                    Some(Zeroizing::new(given))
                }
                None => ask_backup_password(),
            };
            let wallet = require_wallet(cli.wallet.as_deref(), "restore");
            match commands::restore(
                wallet,
                Path::new(backup),
                password.as_deref().map(|p| p.as_str()),
            ) {
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
        Some(Commands::Backups { backup_folder }) => {
            match commands::backups(Path::new(backup_folder)) {
                Ok(lines) => {
                    for line in lines {
                        println!("{}", line);
                    }
                }
                Err(e) => fail(&e),
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("iwcli").chain(args.iter().copied()))
    }

    #[test]
    fn clap_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn no_arguments_starts_the_tui_without_wallet() {
        let cli = parse(&[]).unwrap();
        assert!(cli.wallet.is_none());
        assert!(cli.command.is_none());
    }

    #[test]
    fn wallet_option_short_and_long() {
        assert_eq!(
            parse(&["-w", "/tmp/w"]).unwrap().wallet.as_deref(),
            Some("/tmp/w")
        );
        assert_eq!(
            parse(&["--wallet", "/tmp/w"]).unwrap().wallet.as_deref(),
            Some("/tmp/w")
        );
    }

    #[test]
    fn backup_command() {
        let cli = parse(&["--wallet", "w", "backup", "--backup-folder", "b"]).unwrap();
        assert_eq!(cli.wallet.as_deref(), Some("w"));
        match cli.command {
            Some(Commands::Backup { backup_folder }) => assert_eq!(backup_folder, "b"),
            _ => panic!("expected backup"),
        }
    }

    #[test]
    fn backup_requires_backup_folder() {
        let err = parse(&["--wallet", "w", "backup"]).err().unwrap();
        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn restore_command_with_and_without_password() {
        match parse(&["-w", "w", "restore", "--backup", "f.zip"])
            .unwrap()
            .command
        {
            Some(Commands::Restore { backup, password }) => {
                assert_eq!(backup, "f.zip");
                assert_eq!(password, None);
            }
            _ => panic!("expected restore"),
        }
        match parse(&[
            "-w",
            "w",
            "restore",
            "--backup",
            "f.zip",
            "--password",
            "pw",
        ])
        .unwrap()
        .command
        {
            Some(Commands::Restore { password, .. }) => {
                assert_eq!(password.as_deref(), Some("pw"))
            }
            _ => panic!("expected restore"),
        }
    }

    #[test]
    fn restore_requires_backup() {
        let err = parse(&["-w", "w", "restore"]).err().unwrap();
        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn backups_command_needs_no_wallet() {
        let cli = parse(&["backups", "--backup-folder", "b"]).unwrap();
        assert!(cli.wallet.is_none());
        assert!(
            matches!(cli.command, Some(Commands::Backups { backup_folder }) if backup_folder == "b")
        );
    }

    #[test]
    fn wallet_option_is_global_to_the_top_level_only() {
        // --wallet goes before the subcommand.
        assert!(parse(&["backup", "--backup-folder", "b", "--wallet", "w"]).is_err());
    }

    #[test]
    fn other_commands_parse() {
        assert!(matches!(
            parse(&["version"]).unwrap().command,
            Some(Commands::Version)
        ));
        assert!(matches!(
            parse(&["unlock"]).unwrap().command,
            Some(Commands::Unlock)
        ));
        assert!(matches!(
            parse(&["lock"]).unwrap().command,
            Some(Commands::Lock)
        ));
        assert!(matches!(
            parse(&["list"]).unwrap().command,
            Some(Commands::List { parent_id: None })
        ));
        assert!(matches!(
            parse(&["show", "abc"]).unwrap().command,
            Some(Commands::Show { item_id }) if item_id == "abc"
        ));
        assert!(matches!(
            parse(&["search", "mail"]).unwrap().command,
            Some(Commands::Search { query }) if query == "mail"
        ));
    }

    #[test]
    fn unknown_command_is_an_error() {
        let err = parse(&["frobnicate"]).err().unwrap();
        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidSubcommand);
    }

    #[test]
    fn require_wallet_returns_the_path() {
        assert_eq!(
            require_wallet(Some("/tmp/w"), "backup"),
            Path::new("/tmp/w")
        );
    }
}
