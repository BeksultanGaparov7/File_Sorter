// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Merlin Hermes

use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub watch_dir: String,
    /// Seconds a file's size must stay unchanged before it is moved.
    pub settle_seconds: f64,
    /// In-progress download extensions that are never touched.
    pub ignore_extensions: Vec<String>,
    /// Destination folder -> extensions. Relative folders live inside the sorted dir.
    pub categories: BTreeMap<String, Vec<String>>,
    /// Category folder -> where "Move out of Downloads" sends it. Key `other` = unsorted files.
    pub move_out: BTreeMap<String, String>,
    pub stale: StaleConfig,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct StaleConfig {
    pub enabled: bool,
    /// A file counts as "rotting" after this many days in Downloads.
    pub after_days: f64,
    /// Don't re-notify about the same file more often than this.
    pub remind_every_hours: f64,
    pub check_every_minutes: u64,
    /// More stale files than this are grouped into one summary notification.
    pub max_individual: usize,
}

fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

impl Default for Config {
    fn default() -> Self {
        let categories = [
            ("D_Videos", &["mp4", "mkv", "webm", "avi", "mov", "m4v", "flv", "wmv", "mpg", "mpeg"][..]),
            ("D_Pictures", &["jpg", "jpeg", "png", "gif", "webp", "bmp", "svg", "heic", "tiff", "avif"]),
            ("D_Audio", &["mp3", "flac", "wav", "ogg", "m4a", "aac", "opus"]),
            (
                "D_Documents",
                &[
                    "pdf", "doc", "docx", "odt", "txt", "md", "rtf", "xls", "xlsx", "ods", "csv", "ppt", "pptx", "odp",
                    "epub",
                ],
            ),
            (
                "D_Archives",
                &["zip", "tar", "gz", "tgz", "xz", "bz2", "zst", "7z", "rar", "tar.gz", "tar.xz", "tar.bz2", "tar.zst"],
            ),
            ("D_Programs", &["appimage", "deb", "rpm", "run", "exe", "msi", "flatpakref"]),
        ];
        let move_out = [
            ("D_Videos", "~/Videos"),
            ("D_Pictures", "~/Pictures"),
            ("D_Audio", "~/Music"),
            ("D_Documents", "~/Documents"),
            ("other", "~/Documents"),
        ];
        Config {
            watch_dir: "~/Downloads".into(),
            settle_seconds: 1.0,
            ignore_extensions: strs(&["part", "crdownload", "download", "tmp", "partial", "!ut", "aria2"]),
            categories: categories.iter().map(|(k, v)| (k.to_string(), strs(v))).collect(),
            move_out: move_out.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            stale: StaleConfig::default(),
        }
    }
}

impl Default for StaleConfig {
    fn default() -> Self {
        StaleConfig {
            enabled: true,
            after_days: 7.0,
            remind_every_hours: 24.0,
            check_every_minutes: 60,
            max_individual: 3,
        }
    }
}

pub fn default_config_path() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| home().join(".config")).join("file-sorter/config.toml")
}

impl Config {
    pub fn load(path: &Path) -> Result<Config, String> {
        if !path.is_file() {
            return Ok(Config::default());
        }
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn watch_dir(&self) -> PathBuf {
        expand(&self.watch_dir)
    }

    pub fn rules(&self, base: &Path) -> Rules {
        let mut ext_to_folder = HashMap::new();
        for (folder, exts) in &self.categories {
            for ext in exts {
                ext_to_folder.insert(norm_ext(ext), folder.clone());
            }
        }
        Rules {
            base: base.to_path_buf(),
            ext_to_folder,
            ignore: self.ignore_extensions.iter().map(|e| norm_ext(e)).collect(),
        }
    }

    /// Where "Move out of Downloads" sends an item from `category` (None = unsorted).
    pub fn move_out_for(&self, category: Option<&str>) -> Option<PathBuf> {
        self.move_out.get(category.unwrap_or("other")).map(|s| expand(s))
    }
}

fn norm_ext(e: &str) -> String {
    e.trim_start_matches('.').to_lowercase()
}

pub enum Class {
    Ignored,
    Known { dest: PathBuf },
    Unknown { ext: Option<String> },
}

pub struct Rules {
    pub base: PathBuf,
    ext_to_folder: HashMap<String, String>,
    ignore: HashSet<String>,
}

impl Rules {
    pub fn dest_for(&self, folder: &str) -> PathBuf {
        let p = expand(folder);
        if p.is_absolute() { p } else { self.base.join(p) }
    }

    /// Category destinations that live inside the base dir (these are scanned for stale files).
    pub fn inner_dests(&self) -> Vec<(String, PathBuf)> {
        let folders: HashSet<&String> = self.ext_to_folder.values().collect();
        folders
            .into_iter()
            .map(|f| (f.clone(), self.dest_for(f)))
            .filter(|(_, d)| d.starts_with(&self.base) && d != &self.base)
            .collect()
    }

    /// Classify by extension, preferring the longest known multi-part one (e.g. `tar.gz`).
    pub fn classify(&self, name: &str) -> Class {
        let lower = name.to_lowercase();
        let parts: Vec<&str> = lower.split('.').collect();
        if parts.len() < 2 || parts[parts.len() - 1].is_empty() {
            return Class::Unknown { ext: None };
        }
        for i in 1..parts.len() {
            let ext = parts[i..].join(".");
            if self.ignore.contains(&ext) {
                return Class::Ignored;
            }
            if let Some(folder) = self.ext_to_folder.get(&ext) {
                return Class::Known { dest: self.dest_for(folder) };
            }
        }
        Class::Unknown { ext: Some(parts[parts.len() - 1].to_string()) }
    }

    pub fn is_ignored(&self, name: &str) -> bool {
        matches!(self.classify(name), Class::Ignored)
    }
}

pub fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
}

pub fn expand(s: &str) -> PathBuf {
    if s == "~" {
        home()
    } else if let Some(rest) = s.strip_prefix("~/") {
        home().join(rest)
    } else {
        PathBuf::from(s)
    }
}

/// Display a path with $HOME shortened to `~`.
pub fn tilde(p: &Path) -> String {
    match p.strip_prefix(home()) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".into(),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => p.display().to_string(),
    }
}

