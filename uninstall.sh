#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Merlin Hermes
set -euo pipefail
systemctl --user disable --now file-sorter.service 2>/dev/null || true
rm -f "$HOME/.config/systemd/user/file-sorter.service" "$HOME/.local/bin/file-sorter"
systemctl --user daemon-reload
echo "Uninstalled (config left at ~/.config/file-sorter/, state at ~/.local/state/file-sorter/)."
