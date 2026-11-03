# iwcli

Command-line interface for IntelliWallet password manager.

## Features

- Dual-panel terminal interface
- Encrypted vault via [iwcore](https://crates.io/crates/iwcore): every item name and field value is encrypted with XChaCha20-Poly1305 (authenticated encryption, unique nonce per value) under a random per-vault key, which is wrapped by a key derived from your master password with Argon2id
- Older (pre-v6) vaults are upgraded to this format once, on the first unlock
- Hierarchical folder organization
- TOTP (2FA) code generation
- Create, edit, and delete items and fields
- Password change functionality
- Backup, restore and backup listing from the command line

## Installation

iwcli is not published on crates.io. Build it from source (Rust 1.90 or newer):

```bash
git clone https://github.com/intellisoftalpin/iwcli.git
cd iwcli
cargo install --path .
```

Or build without installing; the binary is `target/release/iwcli`:

```bash
cargo build --release
```

## Usage

```bash
# Open existing wallet
iwcli --wallet ~/MyWallet

# Create new wallet (if path doesn't exist)
iwcli --wallet ~/NewWallet
```

### Backups

```bash
# Create a backup ZIP of the wallet in a folder
iwcli --wallet ~/MyWallet backup --backup-folder ~/WalletBackups

# List the backups in a folder
iwcli backups --backup-folder ~/WalletBackups

# Restore a backup into a wallet folder (replaces its database)
iwcli --wallet ~/MyWallet restore --backup ~/WalletBackups/<file>.zip
```

A backup is checked before anything is restored: it must hold a valid wallet database that is not newer than this version, within iwcore's size limits; anything else is refused and the wallet folder is left untouched. `restore --password <password>` also verifies the password on a temporary copy first. Note that a password given on the command line can end up in your shell history.

## Keyboard Shortcuts

### Navigation

| Key | Action |
|-----|--------|
| `Up` / `k` | Move selection up |
| `Down` / `j` | Move selection down |
| `Left` / `h` | Switch to left panel / Go back |
| `Right` / `l` | Switch to right panel / Enter folder |
| `Tab` | Toggle between panels |
| `Enter` | Enter folder / View field details |
| `Backspace` | Go to parent folder |

### Actions

| Key | Action |
|-----|--------|
| `N` / `F2` | New item (left panel) / New field (right panel) |
| `E` / `F4` | Edit selected item or field |
| `C` / `F7` | Create new folder |
| `D` / `F8` | Delete selected item or field |
| `P` | Change wallet password |
| `Q` | Quit application |

### Dialog Navigation

| Key | Action |
|-----|--------|
| `Tab` / `Up` / `Down` | Switch between input fields |
| `Enter` | Confirm action |
| `Esc` | Cancel / Close dialog |

### Login Screen

| Key | Action |
|-----|--------|
| `Enter` | Unlock wallet |
| `Esc` / `Q` | Quit |

## Dependencies

- [iwcore](https://crates.io/crates/iwcore) - IntelliWallet core library

## License

MIT License - see [LICENSE](LICENSE) for details.
