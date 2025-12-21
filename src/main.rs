//! IntelliWallet CLI
//!
//! Command-line interface for the IntelliWallet password manager.

mod tui;

use clap::{Parser, Subcommand};

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
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Version) => {
            println!("iwcli version {}", env!("CARGO_PKG_VERSION"));
            println!("iwcore version {}", iwcore::DB_VERSION);
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
