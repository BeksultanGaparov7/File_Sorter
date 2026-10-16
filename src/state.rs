// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Merlin Hermes

//! Persistent state for stale-file reminders: which files to never mention
//! again, and when each file was last notified about.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Mutex;

static LOCK: Mutex<()> = Mutex::new(());

#[derive(Default, Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    pub kept: BTreeSet<PathBuf>,
    /// Path -> unix time of the last reminder.
    #[serde(default)]
    pub notified: BTreeMap<PathBuf, u64>,
}

pub fn state_path() -> PathBuf {
    dirs::state_dir().unwrap_or_else(|| crate::config::home().join(".local/state")).join("file-sorter/state.json")
}

impl State {
    fn load() -> State {
        std::fs::read_to_string(state_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    fn save(&self) {
        let path = state_path();
        let write = || -> std::io::Result<()> {
            std::fs::create_dir_all(path.parent().unwrap())?;
            let tmp = path.with_extension("json.tmp");
            std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
            std::fs::rename(tmp, &path)
        };
        if let Err(e) = write() {
            eprintln!("WARN could not save state {}: {e}", path.display());
        }
    }

    /// Read-modify-write the state file, dropping entries for files that no longer exist.
    pub fn update<R>(f: impl FnOnce(&mut State) -> R) -> R {
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut state = State::load();
        state.kept.retain(|p| p.exists());
        state.notified.retain(|p, _| p.exists());
        let result = f(&mut state);
        state.save();
        result
    }

    pub fn read() -> State {
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        State::load()
    }
}
