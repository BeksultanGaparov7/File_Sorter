# Contributing

Thanks for your interest! Bug reports, ideas and pull requests are all welcome.
Everyone taking part is expected to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Reporting bugs and asking for features

Use the [issue tracker](https://github.com/BeksultanGaparov7/File_Sorter/issues). For bugs, please include `file-sorter -V`, your distro,
desktop and notification daemon, and the log from `journalctl --user -u file-sorter -n 50`.
Security issues go through [private reporting](SECURITY.md) instead.

## Development

You need a Rust toolchain, version 1.89 or newer (`rustup` or your distro's `rust` package).

```sh
git clone https://github.com/BeksultanGaparov7/File_Sorter.git
cd File_Sorter
cargo build
cargo test
```

To try your build without touching the installed service, use a scratch folder:

```sh
mkdir -p /tmp/fs-test/dl
printf 'watch_dir = "/tmp/fs-test/dl"\n' > /tmp/fs-test/config.toml
cargo run -- -c /tmp/fs-test/config.toml --watch      # then drop files into /tmp/fs-test/dl
```

To install your build and restart the service: `./install.sh`.

### Layout

| File | Purpose |
|---|---|
| `src/main.rs` | CLI flags, `-help` guide, `--sort / --stale / --review / --rules` |
| `src/config.rs` | Config file, defaults, extension → folder rules |
| `src/sorter.rs` | Moving files, name clashes, waiting for downloads to finish |
| `src/watch.rs` | `--watch`: inotify loop used by the systemd service |
| `src/stale.rs` | Finding old files, reminder notifications, trash / move / keep |
| `src/state.rs` | Remembering reminders and kept files (`~/.local/state/file-sorter`) |
| `src/notify.rs` | Desktop notifications over D-Bus |

## Pull requests

1. Fork the repo and create a branch off `main`.
2. Keep changes focused. One feature or fix per PR is easiest to review.
3. Before pushing, run:
   ```sh
   cargo fmt
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```
4. Add tests for new logic where practical, and a line to `CHANGELOG.md` under **Unreleased**.
5. If you add an option, update `config.toml`, the `-help` guide in `src/main.rs`, and the README.

New source files should start with:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later
```

## License

By contributing, you agree that your contributions are licensed under the
[GNU General Public License v3.0 or later](LICENSE), the same as the rest of the project.
