// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Merlin Hermes

//! `--watch`: the background service. Sorts new arrivals and runs stale reminders.

use crate::config::{Config, tilde};
use crate::notify;
use crate::sorter::{Outcome, sort_file, wait_until_settled};
use crate::stale;
use inotify::{EventMask, Inotify, WatchMask};
use notify_rust::Urgency;
use std::path::Path;
use std::sync::Arc;

pub fn run(cfg: Config) -> Result<(), String> {
    let base = cfg.watch_dir();
    std::fs::create_dir_all(&base).map_err(|e| format!("{}: {e}", base.display()))?;
    let cfg = Arc::new(cfg);
    let rules = Arc::new(cfg.rules(&base));

    let mut inotify = Inotify::init().map_err(|e| format!("inotify: {e}"))?;
    // CLOSE_WRITE: file finished being written in place.
    // MOVED_TO: browsers rename *.part / *.crdownload to the final name.
    inotify
        .watches()
        .add(&base, WatchMask::CLOSE_WRITE | WatchMask::MOVED_TO)
        .map_err(|e| format!("watch {}: {e}", base.display()))?;
    eprintln!("INFO watching {}", base.display());

    if cfg.stale.enabled {
        stale::run_reminders(Arc::clone(&cfg), Arc::clone(&rules));
    }

    let mut buffer = [0u8; 4096];
    loop {
        let events = inotify.read_events_blocking(&mut buffer).map_err(|e| format!("inotify: {e}"))?;
        let names: Vec<_> = events
            .filter(|e| !e.mask.contains(EventMask::ISDIR))
            .filter_map(|e| e.name.map(|n| n.to_owned()))
            .collect();
        for name in names {
            handle(&cfg, &rules, &base.join(name));
        }
    }
}

fn handle(cfg: &Config, rules: &crate::config::Rules, path: &Path) {
    let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
    if rules.is_ignored(&name) || !wait_until_settled(path, cfg.settle_seconds) {
        return;
    }
    match sort_file(rules, path, false) {
        Outcome::Moved { to } => {
            let dest = to.parent().map(tilde).unwrap_or_default();
            eprintln!("INFO moved {name} -> {}", to.display());
            let shown = to.file_name().unwrap_or_default().to_string_lossy();
            notify::send("File sorted", &format!("{shown}\n→ {dest}"), "folder-download", Urgency::Low);
        }
        Outcome::Unknown { ext } => {
            eprintln!("INFO unknown format: {name}");
            let reason = match ext {
                Some(ext) => format!("No rule for “.{ext}”"),
                None => "File has no extension".into(),
            };
            notify::send(
                "Unknown file format",
                &format!(
                    "{name}\n{reason} — left in {}.\nAdd it to ~/.config/file-sorter/config.toml",
                    tilde(&rules.base)
                ),
                "dialog-question",
                Urgency::Normal,
            );
        }
        Outcome::Failed(e) => {
            eprintln!("ERROR moving {name}: {e}");
            notify::send(
                "File Sorter error",
                &format!("Could not move {name}:\n{e}"),
                "dialog-error",
                Urgency::Critical,
            );
        }
        Outcome::Skipped => {}
    }
}
