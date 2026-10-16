#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Merlin Hermes
set -euo pipefail

version="${1:?usage: changelog-section.sh VERSION}"
version="${version#v}"

root="$(cd "$(dirname "$0")/../.." && pwd)"
changelog="$root/CHANGELOG.md"

[[ -f "$changelog" ]] || { echo "changelog-section: missing $changelog" >&2; exit 1; }

awk -v ver="$version" '
  !found && $0 ~ "^## \\[" ver "\\]" { found = 1; next }
  found && (/^## / || /^\[[^]]+\]:/) { exit }
  found {
    if ($0 == "" && !started) next
    started = 1
    print
  }
  END { if (!found) exit 1 }
' "$changelog" || {
  echo "changelog-section: no section for version $version in CHANGELOG.md" >&2
  exit 1
}
