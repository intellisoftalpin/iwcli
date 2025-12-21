# iwcli

Command-line interface for IntelliWallet password manager.

## Features

- Dual-panel terminal interface
- AES-256 encrypted password storage
- Hierarchical folder organization
- TOTP (2FA) code generation
- Create, edit, and delete items and fields
- Password change functionality

## Installation

```bash
cargo install iwcli
```

Or build from source:

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
