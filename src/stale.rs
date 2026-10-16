// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Merlin Hermes

//! Finding files that have been sitting in Downloads too long, and acting on them.

use crate::config::{Config, Rules, tilde};
use crate::notify;
use crate::sorter::{is_hidden, move_path, unique_path};
use crate::state::State;
use notify_rust::Urgency;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, thread, time::Duration};

pub struct StaleItem {
    pub path: PathBuf,
    pub age_secs: u64,
    /// Category folder it was sorted into; None = unsorted, directly in the dir.
    pub category: Option<String>,
}

pub enum Action {
    Trash,
    MoveOut,
    Keep,
}

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn format_age(secs: u64) -> String {
    let days = secs / 86_400;
    match days {
        0 => format!("{} h", secs / 3600),
        1 => "1 day".into(),
        d => format!("{d} days"),
    }
}

/// Everything in `base` (top level and inside its category folders) older than `stale.after_days`.
/// Age is the file's ctime, which is reset when it lands in (or is sorted within) the folder.
pub fn find_stale(cfg: &Config, rules: &Rules) -> Vec<StaleItem> {
    let threshold = (cfg.stale.after_days * 86_400.0) as u64;
    let now = now();
    let inner = rules.inner_dests();
    let mut items = Vec::new();

    let mut scan = |dir: &Path, category: Option<&str>| {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if is_hidden(&path) || rules.is_ignored(&name) {
                continue;
            }
            // Category folders (and their parents) are scanned separately, not reported.
            if inner.iter().any(|(_, d)| d.starts_with(&path)) {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            let age = now.saturating_sub(meta.ctime().max(0) as u64);
            if age >= threshold {
                items.push(StaleItem { path, age_secs: age, category: category.map(String::from) });
            }
        }
    };

    scan(&rules.base, None);
    for (folder, dest) in &inner {
        scan(dest, Some(folder));
    }
    items.sort_by_key(|i| std::cmp::Reverse(i.age_secs));
    items
}

/// Perform `action` on `item`; returns a short human-readable result.
pub fn apply(cfg: &Config, item: &StaleItem, action: Action) -> Result<String, String> {
    let name = item.path.file_name().unwrap_or_default().to_string_lossy().into_owned();
    match action {
        Action::Trash => {
            trash::delete(&item.path).map_err(|e| format!("Could not trash {name}: {e}"))?;
            Ok(format!("Moved to Trash: {name}"))
        }
        Action::MoveOut => {
            let dest = cfg
                .move_out_for(item.category.as_deref())
                .ok_or_else(|| format!("No move_out folder configured for {name}"))?;
            fs::create_dir_all(&dest).map_err(|e| format!("{}: {e}", dest.display()))?;
            let target = unique_path(&dest.join(&name));
            move_path(&item.path, &target).map_err(|e| format!("Could not move {name}: {e}"))?;
            Ok(format!("Moved {name} → {}", tilde(&dest)))
        }
        Action::Keep => {
            State::update(|s| s.kept.insert(item.path.clone()));
            Ok(format!("Keeping {name} (no more reminders)"))
        }
    }
}

/// Background loop used by `--watch`.
pub fn run_reminders(cfg: Arc<Config>, rules: Arc<Rules>) {
    thread::spawn(move || {
        loop {
            check_and_notify(&cfg, &rules);
            thread::sleep(Duration::from_secs(cfg.stale.check_every_minutes.max(1) * 60));
        }
    });
}

fn check_and_notify(cfg: &Arc<Config>, rules: &Rules) {
    let remind_every = (cfg.stale.remind_every_hours * 3600.0) as u64;
    let now = now();
    let stale = find_stale(cfg, rules);
    // Pick the items due for a reminder and record that we're reminding now.
    let due: Vec<StaleItem> = State::update(|s| {
        let due: Vec<StaleItem> = stale
            .into_iter()
            .filter(|i| !s.kept.contains(&i.path))
            .filter(|i| s.notified.get(&i.path).is_none_or(|&t| now.saturating_sub(t) >= remind_every))
            .collect();
        for i in &due {
            s.notified.insert(i.path.clone(), now);
        }
        due
    });
    if due.is_empty() {
        return;
    }
    let dir_name = tilde(&rules.base);
    eprintln!("INFO {} stale item(s) in {dir_name}", due.len());

    if due.len() > cfg.stale.max_individual {
        let mut body: Vec<String> = due.iter().take(5).map(|i| display_name(rules, i)).collect();
        if due.len() > 5 {
            body.push(format!("…and {} more", due.len() - 5));
        }
        let base = rules.base.clone();
        notify::send_with_actions(
            &format!("{} files rotting in {dir_name}", due.len()),
            &format!("Older than {} days:\n{}", cfg.stale.after_days, body.join("\n")),
            "user-trash",
            &[("review", "Review in terminal".into())],
            move |action| match action {
                "default" => open(&base),
                "review" => review_in_terminal(),
                _ => {}
            },
        );
        return;
    }

    for item in due {
        let mut actions = vec![("trash", "Move to Trash".to_string())];
        if let Some(dest) = cfg.move_out_for(item.category.as_deref()) {
            actions.push(("move", format!("Move to {}", tilde(&dest))));
        }
        actions.push(("keep", "Keep".into()));
        let cfg = Arc::clone(cfg);
        let folder = item.path.parent().map(Path::to_path_buf).unwrap_or_default();
        notify::send_with_actions(
            &format!("Sitting in {dir_name} for {}", format_age(item.age_secs)),
            &display_name(rules, &item),
            "user-trash",
            &actions,
            move |action| {
                let result = match action {
                    "trash" => apply(&cfg, &item, Action::Trash),
                    "move" => apply(&cfg, &item, Action::MoveOut),
                    "keep" => apply(&cfg, &item, Action::Keep),
                    "default" => return open(&folder),
                    _ => return,
                };
                match result {
                    Ok(msg) => {
                        eprintln!("INFO {msg}");
                        notify::send("Done", &msg, "emblem-ok", Urgency::Low);
                    }
                    Err(msg) => {
                        eprintln!("ERROR {msg}");
                        notify::send("File Sorter error", &msg, "dialog-error", Urgency::Critical);
                    }
                }
            },
        );
    }
}

pub fn display_name(rules: &Rules, item: &StaleItem) -> String {
    let rel = item.path.strip_prefix(&rules.base).unwrap_or(&item.path).display().to_string();
    if item.path.is_dir() { format!("{rel}/") } else { rel }
}

fn open(path: &Path) {
    let _ = std::process::Command::new("xdg-open").arg(path).status();
}

fn review_in_terminal() {
    let exe = std::env::current_exe().unwrap_or_else(|_| "file-sorter".into());
    let _ = std::process::Command::new("xdg-terminal-exec").arg(exe).arg("--review").status();
}
