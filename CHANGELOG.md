# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-10-16

### Added
- Background service (`file-sorter --watch`, systemd user unit) that sorts finished downloads
  in `~/Downloads` into category folders and sends a desktop notification.
- "Unknown file format" notification for files without a matching rule. These files stay in place.
- Reminders for files left in Downloads too long, with **Move to Trash**, **Move to …** and **Keep** buttons.
  More than a few stale files are grouped into one summary notification.
- CLI: `-s/--sort`, `-t/--stale`, `-r/--review`, `-n/--dry-run`, `--rules`, `--status`, `--paths`, `-help` guide.
- Folder shortcuts: `@downloads`, `@home`, `@desktop`, `@documents`, `@videos`, `@pictures`, `@music`.
- TOML config at `~/.config/file-sorter/config.toml`.
- `install.sh` / `uninstall.sh`. The installer puts the binary on your PATH.

[Unreleased]: https://github.com/BeksultanGaparov7/File_Sorter/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/BeksultanGaparov7/File_Sorter/releases/tag/v0.1.0
