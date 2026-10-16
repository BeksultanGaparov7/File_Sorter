# File Sorter

[![CI](https://github.com/BeksultanGaparov7/File_Sorter/actions/workflows/ci.yml/badge.svg)](https://github.com/BeksultanGaparov7/File_Sorter/actions/workflows/ci.yml)
[![License: GPL v3+](https://img.shields.io/badge/License-GPLv3%2B-blue.svg)](LICENSE)

**Keeps your `~/Downloads` folder from turning into a junk drawer.**

File Sorter is a small Linux background service. When a download finishes, it moves the file
into a folder for its type (`.mp4` → `D_Videos`, `.pdf` → `D_Documents`, …) and tells you with
a desktop notification. Files it doesn't recognise stay put, and you get a notification about them.
Anything left in Downloads for more than a week gets a reminder with
**Move to Trash**, **Move to ~/Videos** and **Keep** buttons.

```
movie.mp4 lands in ~/Downloads
        │
        ▼
  ~/Downloads/D_Videos/movie.mp4      🔔 "File sorted: movie.mp4 → ~/Downloads/D_Videos"

weird.xyz lands in ~/Downloads        🔔 "Unknown file format: no rule for .xyz"

7 days later                          🔔 "Sitting in ~/Downloads for 7 days: D_Videos/movie.mp4"
                                         [Move to Trash] [Move to ~/Videos] [Keep]
```

## Features

- **Automatic sorting** of finished downloads. In-progress `.part` / `.crdownload` files are ignored.
- **Desktop notifications** for every sorted file, unknown format or error.
- **Rot reminders** for files sitting in Downloads too long, with buttons to act on them right away.
- **Never overwrites.** Name clashes become `name (1).ext`. "Trash" uses the normal freedesktop trash, so files can be restored.
- **CLI** for sorting any folder on demand, previewing, and reviewing old files interactively.
- **One small binary**, written in Rust. It talks to inotify and D-Bus directly, makes no network connections and collects no telemetry.

## Requirements

- Linux with systemd (user services) and a desktop notification daemon
  (mako, dunst, swaync, GNOME, KDE, …). For the reminder buttons, the daemon must support
  notification *actions*. Without them you can still deal with old files using `file-sorter -r`.
- To build from source: Rust **1.89+** ([rustup](https://rustup.rs) or your distro's `rust` package).

## Installation

### From source

```sh
git clone https://github.com/BeksultanGaparov7/File_Sorter.git
cd File_Sorter
./install.sh
```

### From a release

Download `file-sorter-vX.Y.Z-x86_64-linux.tar.gz` from
[Releases](https://github.com/BeksultanGaparov7/File_Sorter/releases), then:

```sh
tar xzf file-sorter-*-x86_64-linux.tar.gz
cd file-sorter-*-x86_64-linux
./install.sh
```

`install.sh` does the following:

1. builds the binary (or uses the prebuilt one) and installs it to `~/.local/bin/file-sorter`
2. adds `~/.local/bin` to your `PATH` in `~/.zshrc` / `~/.bashrc` if it isn't there already
3. installs the default config to `~/.config/file-sorter/config.toml` (an existing config is kept)
4. installs and starts the `file-sorter` systemd **user** service

### Uninstall

```sh
./uninstall.sh
```

Your config (`~/.config/file-sorter/`) and state (`~/.local/state/file-sorter/`) are left in place.

## Usage

The service runs on its own once installed. The CLI is for doing things by hand:

```sh
file-sorter -help                 # full guide with examples
file-sorter -s                    # sort everything currently in ~/Downloads
file-sorter -s @downloads         # same, with the folder spelled out
file-sorter -s ~/Desktop -n       # preview sorting another folder (dry run)
file-sorter -t                    # list files sitting in Downloads too long
file-sorter -r                    # go through them: (t)rash / (m)ove out / (k)eep / (s)kip
file-sorter --rules               # show which extensions go where
file-sorter --status              # is the service running?
file-sorter --paths               # where config, state and the binary live
```

| Flag | Meaning |
|---|---|
| `-s, --sort [DIR]` | Sort all files in DIR now (default `@downloads`) |
| `-t, --stale [DIR]` | List files older than the reminder threshold |
| `-r, --review [DIR]` | Interactively trash, move out or keep old files |
| `-n, --dry-run` | With `--sort`: only show what would happen |
| `-w, --watch` | Run the watcher in the foreground (this is what the service runs) |
| `--rules`, `--status`, `--paths` | Show rules / service status / file locations |
| `-c, --config FILE` | Use a different config file |
| `-h, -help, --help` | Show the guide |
| `-V, --version` | Show version and license |

**Folder shortcuts:** `@downloads` `@home` `@desktop` `@documents` `@videos` `@pictures` `@music`.
Any normal path works too.

## Configuration

Everything lives in `~/.config/file-sorter/config.toml`. The [default config](config.toml)
has comments for every option. After editing, apply it with:

```sh
systemctl --user restart file-sorter
```

```toml
watch_dir = "~/Downloads"

[categories]                     # folder = extensions it receives
D_Videos   = ["mp4", "mkv", "webm"]
D_Pictures = ["jpg", "png"]
"~/Books"  = ["epub"]            # destinations can also be outside Downloads

[stale]
after_days = 7                   # when a file counts as "rotting"
remind_every_hours = 24
max_individual = 3               # more than this → one summary notification

[move_out]                       # target of the "Move to …" button
D_Videos = "~/Videos"
other    = "~/Documents"         # for files that were never sorted
```

## How it works

- `file-sorter --watch` uses **inotify** to watch the top level of `~/Downloads` for files that
  finished writing (`CLOSE_WRITE`) or were renamed into place (`MOVED_TO`, which is how browsers
  finish downloads). It waits until the file size stops changing, then moves the file.
- Reminders run once an hour. A file's age is how long ago it arrived in Downloads or was sorted
  within it (its *ctime*). Reminders cover loose files and the contents of the category folders.
  Pressing **Keep** stops reminders for that file permanently.
  Reminder history is stored in `~/.local/state/file-sorter/state.json`.
- Notifications go straight to the freedesktop notification service over D-Bus.

Logs: `journalctl --user -u file-sorter -f`

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for how to build, test and
send a pull request, and please follow the [Code of Conduct](CODE_OF_CONDUCT.md).
Found a security problem? See [SECURITY.md](SECURITY.md).

## License

Copyright (C) 2026 Merlin Hermes

This program is free software: you can redistribute it and/or modify it under the terms of the
GNU General Public License as published by the Free Software Foundation, either version 3 of the
License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but **WITHOUT ANY WARRANTY**;
without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
See the [GNU General Public License](LICENSE) for more details.
