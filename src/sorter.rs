// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Merlin Hermes

use crate::config::{Class, Rules};
use std::path::{Path, PathBuf};
use std::{fs, io, thread, time::Duration};

pub enum Outcome {
    Moved { to: PathBuf },
    Unknown { ext: Option<String> },
    Skipped,
    Failed(String),
}

pub fn is_hidden(path: &Path) -> bool {
    path.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.'))
}

/// Move one regular file into its category folder (or report it as unknown).
pub fn sort_file(rules: &Rules, path: &Path, dry_run: bool) -> Outcome {
    let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else {
        return Outcome::Skipped;
    };
    let is_file = fs::symlink_metadata(path).is_ok_and(|m| m.is_file());
    if !is_file || is_hidden(path) {
        return Outcome::Skipped;
    }
    match rules.classify(&name) {
        Class::Ignored => Outcome::Skipped,
        Class::Unknown { ext } => Outcome::Unknown { ext },
        Class::Known { dest, .. } => {
            if dry_run {
                return Outcome::Moved { to: dest.join(&name) };
            }
            let result = fs::create_dir_all(&dest).and_then(|_| {
                let target = unique_path(&dest.join(&name));
                move_path(path, &target).map(|_| target)
            });
            match result {
                Ok(to) => Outcome::Moved { to },
                Err(e) => Outcome::Failed(e.to_string()),
            }
        }
    }
}

/// Wait until the file's size stops changing. Returns false if it disappeared.
pub fn wait_until_settled(path: &Path, settle_seconds: f64) -> bool {
    let size = |p: &Path| fs::metadata(p).map(|m| m.len()).ok();
    let Some(mut last) = size(path) else { return false };
    loop {
        thread::sleep(Duration::from_secs_f64(settle_seconds));
        match size(path) {
            None => return false,
            Some(s) if s == last => return true,
            Some(s) => last = s,
        }
    }
}

/// `name.ext` -> `name (1).ext`, `name (2).ext`, ... until the path is free.
pub fn unique_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let mut split = name.rfind('.').filter(|&i| i > 0).unwrap_or(name.len());
    if name[..split].to_lowercase().ends_with(".tar") {
        split -= 4; // keep `.tar.gz` together
    }
    let (stem, ext) = name.split_at(split);
    (1..).map(|n| path.with_file_name(format!("{stem} ({n}){ext}"))).find(|p| !p.exists()).unwrap()
}

/// Rename, falling back to `mv` when crossing filesystems.
pub fn move_path(src: &Path, dst: &Path) -> io::Result<()> {
    match fs::rename(src, dst) {
        Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
            let status = std::process::Command::new("mv").arg("-nT").arg("--").arg(src).arg(dst).status()?;
            if status.success() { Ok(()) } else { Err(io::Error::other("mv failed")) }
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn tempdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("file-sorter-test-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn unique_path_numbers_before_extension() {
        let dir = tempdir("unique");
        for name in ["a.mp4", "a (1).mp4", "b.tar.gz", "README"] {
            fs::write(dir.join(name), "x").unwrap();
        }
        assert_eq!(unique_path(&dir.join("a.mp4")), dir.join("a (2).mp4"));
        assert_eq!(unique_path(&dir.join("b.tar.gz")), dir.join("b (1).tar.gz"));
        assert_eq!(unique_path(&dir.join("README")), dir.join("README (1)"));
        assert_eq!(unique_path(&dir.join("new.txt")), dir.join("new.txt"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sort_file_moves_known_and_leaves_unknown() {
        let dir = tempdir("sort");
        let rules = Config::default().rules(&dir);
        fs::write(dir.join("movie.mp4"), "x").unwrap();
        fs::write(dir.join("weird.xyz"), "x").unwrap();
        fs::write(dir.join(".hidden.mp4"), "x").unwrap();

        assert!(matches!(sort_file(&rules, &dir.join("movie.mp4"), false), Outcome::Moved { .. }));
        assert!(dir.join("D_Videos/movie.mp4").is_file());
        assert!(matches!(sort_file(&rules, &dir.join("weird.xyz"), false), Outcome::Unknown { .. }));
        assert!(dir.join("weird.xyz").is_file());
        assert!(matches!(sort_file(&rules, &dir.join(".hidden.mp4"), false), Outcome::Skipped));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn dry_run_moves_nothing() {
        let dir = tempdir("dry");
        let rules = Config::default().rules(&dir);
        fs::write(dir.join("song.mp3"), "x").unwrap();
        assert!(matches!(sort_file(&rules, &dir.join("song.mp3"), true), Outcome::Moved { .. }));
        assert!(dir.join("song.mp3").is_file());
        assert!(!dir.join("D_Audio").exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
