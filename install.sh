#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Merlin Hermes
set -euo pipefail
cd "$(dirname "$0")"

bin_dir="$HOME/.local/bin"
conf="${XDG_CONFIG_HOME:-$HOME/.config}/file-sorter/config.toml"

if [[ -f Cargo.toml ]]; then
  # Source checkout: build it.
  command -v cargo >/dev/null || { echo "Missing: cargo (install rust, https://rustup.rs)"; exit 1; }
  echo "Building..."
  cargo build --release --quiet
  binary=target/release/file-sorter
else
  # Release tarball: prebuilt binary next to this script.
  binary=./file-sorter
  [[ -x "$binary" ]] || { echo "No Cargo.toml and no prebuilt ./file-sorter found"; exit 1; }
fi
install -Dm755 "$binary" "$bin_dir/file-sorter"
echo "Installed: $bin_dir/file-sorter"

# Make sure ~/.local/bin is on PATH for future shells.
case ":$PATH:" in
  *":$bin_dir:"*) echo "PATH: $bin_dir already on PATH" ;;
  *)
    line='export PATH="$HOME/.local/bin:$PATH"'
    case "$(basename "${SHELL:-bash}")" in
      zsh) rc="$HOME/.zshrc" ;;
      fish) rc=""; fish -c "fish_add_path $bin_dir" ;;
      *) rc="$HOME/.bashrc" ;;
    esac
    if [[ -n "$rc" ]] && ! grep -qsF "$line" "$rc"; then
      printf '\n# Added by file-sorter install.sh\n%s\n' "$line" >> "$rc"
      echo "PATH: added $bin_dir to $rc (open a new terminal or run: source $rc)"
    fi
    ;;
esac

if [[ ! -f "$conf" ]]; then
  install -Dm644 config.toml "$conf"
  echo "Config installed: $conf"
elif ! grep -q '^\[stale\]' "$conf"; then
  # Config from the old version: keep a backup, install the new format.
  cp "$conf" "$conf.bak"
  install -Dm644 config.toml "$conf"
  echo "Config upgraded: $conf (old one saved as config.toml.bak)"
else
  echo "Keeping existing config: $conf"
fi

install -Dm644 file-sorter.service "$HOME/.config/systemd/user/file-sorter.service"
systemctl --user daemon-reload
systemctl --user enable file-sorter.service --quiet
systemctl --user restart file-sorter.service
echo
echo "File Sorter is running. Try: file-sorter -help"
