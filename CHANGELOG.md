# Changelog

## 0.1.3

### Changed

- iwcore 0.6.0. Old (pre-v6) vaults are upgraded once to the Argon2id +
  XChaCha20-Poly1305 format on the first unlock; the TUI now reports the
  upgrade, records that could not be decrypted, and the old-format copy
  `nswallet.pre-v6.bak` left in the wallet folder.
- Dependencies at their latest releases: clap 4.6, ratatui 0.30 (crossterm
  0.29 through `ratatui::crossterm`), totp-rs 6, tempfile 3.27. The unused
  rpassword, rustyline, colored and serde_json dependencies are removed.
- `version` prints the wallet database format as such (it was labelled as the
  iwcore version).

### Security

- `restore` checks the backup with iwcore's untrusted-backup path before the
  wallet folder is touched (SQLite header, integrity check, real wallet
  tables, read-only connection, 256 MiB and compression-ratio limits) and
  verifies `--password` on a temporary copy without migrating or writing it.
  Refused backups leave the wallet untouched.
- Typed passwords (unlock, create wallet, change password, `restore
  --password`) are kept in zeroizing buffers and wiped as soon as they have
  been used; the dialog input buffer is wiped when a dialog closes.

### Fixed

- TOTP codes for `otpauth://` URLs with secrets shorter than 128 bits; the
  remaining seconds follow the URL's period.
- clippy findings.

### Added

- Unit tests (TOTP, CLI parsing, backup/restore/backups on a temp wallet,
  TUI app state).
- CI on GitHub-hosted runners (Linux, macOS, Windows): build, test, clippy,
  MSRV check, rustfmt, cargo audit.

## 0.1.2

- `backup`, `restore` and `backups` commands; iwcore 0.1.5.
