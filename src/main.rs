// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Merlin Hermes

mod config;
mod notify;
mod sorter;
mod stale;
mod state;
mod watch;

use clap::{ArgAction, CommandFactory, Parser};
use config::{Config, default_config_path, expand, tilde};
use sorter::Outcome;
use stale::{Action, find_stale, format_age};
use state::State;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

const GUIDE: &str = "\
EXAMPLES:
  file-sorter -s                    Sort everything currently in ~/Downloads
  file-sorter -s @downloads         Same, with the folder spelled out
  file-sorter -s ~/Desktop -n       Preview sorting ~/Desktop (nothing is moved)
  file-sorter -t                    List files that have been sitting in Downloads too long
  file-sorter -r                    Go through old files one by one: trash / move out / keep
  file-sorter --rules               Show which extensions go to which folder
  file-sorter --status              Check that the background service is running

FOLDER SHORTCUTS (for -s, -t, -r):
  @downloads  @home  @desktop  @documents  @videos  @pictures  @music
  Any normal path works too:  file-sorter -s ~/Desktop

BACKGROUND SERVICE (installed by install.sh, runs `file-sorter --watch`):
  • A new file lands in ~/Downloads  →  moved to its folder (e.g. .mp4 → D_Videos) + notification
  • Unknown file type               →  notification, file stays where it is
  • Files older than 7 days          →  reminder with [Move to Trash] [Move to ~/Videos] [Keep]
                                        (\"Keep\" stops reminders for that file)

  systemctl --user restart file-sorter     apply config changes
  journalctl --user -u file-sorter -f      follow the log

CONFIG:
  ~/.config/file-sorter/config.toml — folders, extensions, reminder timing, move-out targets.
  `file-sorter --paths` shows every file the app uses.";

const LONG_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "\nCopyright (C) 2026 Merlin Hermes\n",
    "License GPLv3+: GNU GPL version 3 or later <https://gnu.org/licenses/gpl.html>.\n",
    "This is free software: you are free to change and redistribute it.\n",
    "There is NO WARRANTY, to the extent permitted by law."
);

#[derive(Parser)]
#[command(
    name = "file-sorter",
    version,
    long_version = LONG_VERSION,
    about = "Sorts your downloads into folders and nags you about files left in Downloads.",
    after_long_help = GUIDE,
    after_help = "Run `file-sorter -help` for the full guide with examples.",
    disable_help_flag = true,
    arg_required_else_help = false
)]
struct Cli {
    /// Sort all files in DIR now [default: @downloads]
    #[arg(short, long, value_name = "DIR", num_args = 0..=1, default_missing_value = "@downloads", group = "mode")]
    sort: Option<String>,

    /// List files that have been in DIR too long [default: @downloads]
    #[arg(short = 't', long, value_name = "DIR", num_args = 0..=1, default_missing_value = "@downloads", group = "mode")]
    stale: Option<String>,

    /// Interactively trash / move out / keep old files in DIR [default: @downloads]
    #[arg(short, long, value_name = "DIR", num_args = 0..=1, default_missing_value = "@downloads", group = "mode")]
    review: Option<String>,

    /// Watch Downloads and sort new files as they arrive (what the service runs)
    #[arg(short, long, group = "mode")]
    watch: bool,

    /// Show which extensions go where, and reminder settings
    #[arg(long, group = "mode")]
    rules: bool,

    /// Show whether the background service is running
    #[arg(long, group = "mode")]
    status: bool,

    /// Show config, state and binary locations
    #[arg(long, group = "mode")]
    paths: bool,

    /// With --sort: only show what would be moved
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Use a different config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Show the full guide (also: -help, help)
    #[arg(short, long, action = ArgAction::HelpLong)]
    help: Option<bool>,
}

fn main() -> ExitCode {
    // Accept `-help` and `help` as well as -h / --help.
    let args = std::env::args_os()
        .enumerate()
        .map(|(i, a)| if a == "-help" || (i == 1 && a == "help") { "--help".into() } else { a });
    let cli = Cli::parse_from(args);

    let config_path = cli.config.clone().unwrap_or_else(default_config_path);
    let cfg = match Config::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Config error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let result = if let Some(dir) = &cli.sort {
        resolve(dir, &cfg).and_then(|d| cmd_sort(&cfg, d, cli.dry_run))
    } else if let Some(dir) = &cli.stale {
        resolve(dir, &cfg).map(|d| cmd_stale(&cfg, d))
    } else if let Some(dir) = &cli.review {
        resolve(dir, &cfg).map(|d| cmd_review(&cfg, d))
    } else if cli.watch {
        watch::run(cfg)
    } else if cli.rules {
        cmd_rules(&cfg);
        Ok(())
    } else if cli.status {
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "status", "file-sorter", "--no-pager", "-n", "10"])
            .status();
        Ok(())
    } else if cli.paths {
        let exe = std::env::current_exe().map(|p| tilde(&p)).unwrap_or_default();
        let exists = if config_path.is_file() { "" } else { "  (not created — using defaults)" };
        println!("Config:      {}{exists}", tilde(&config_path));
        println!("State:       {}", tilde(&state::state_path()));
        println!("Binary:      {exe}");
        println!("Watching:    {}", tilde(&cfg.watch_dir()));
        println!("Service:     ~/.config/systemd/user/file-sorter.service");
        Ok(())
    } else {
        let _ = Cli::command().print_help();
        Ok(())
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Turn `@downloads`, `~/x` or a plain path into an existing directory.
fn resolve(arg: &str, cfg: &Config) -> Result<PathBuf, String> {
    let path = match arg.to_lowercase().as_str() {
        "@downloads" | "@dl" => Some(cfg.watch_dir()),
        "@home" => dirs::home_dir(),
        "@desktop" => dirs::desktop_dir(),
        "@documents" | "@docs" => dirs::document_dir(),
        "@videos" => dirs::video_dir(),
        "@pictures" => dirs::picture_dir(),
        "@music" => dirs::audio_dir(),
        a if a.starts_with('@') => {
            return Err(format!(
                "unknown shortcut '{arg}'. Use one of: @downloads @home @desktop @documents @videos @pictures @music"
            ));
        }
        _ => Some(expand(arg)),
    }
    .ok_or_else(|| format!("could not find the folder for {arg}"))?;
    let path = path.canonicalize().map_err(|e| format!("{}: {e}", path.display()))?;
    if !path.is_dir() {
        return Err(format!("{} is not a directory", path.display()));
    }
    Ok(path)
}

fn cmd_sort(cfg: &Config, dir: PathBuf, dry_run: bool) -> Result<(), String> {
    let rules = cfg.rules(&dir);
    let mut entries: Vec<PathBuf> =
        std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?.flatten().map(|e| e.path()).collect();
    entries.sort();

    println!("{} {}", if dry_run { "Preview of sorting" } else { "Sorting" }, tilde(&dir));
    let (mut moved, mut unknown, mut failed) = (0, 0, 0);
    for path in entries {
        let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        match sorter::sort_file(&rules, &path, dry_run) {
            Outcome::Moved { to } => {
                moved += 1;
                let rel = to.strip_prefix(&dir).map(|p| p.display().to_string()).unwrap_or_else(|_| tilde(&to));
                println!("  ✓ {name}  →  {rel}");
            }
            Outcome::Unknown { ext } => {
                unknown += 1;
                let why = ext.map(|e| format!("no rule for .{e}")).unwrap_or("no extension".into());
                println!("  ? {name}  ({why}, left in place)");
            }
            Outcome::Failed(e) => {
                failed += 1;
                println!("  ✗ {name}  ({e})");
            }
            Outcome::Skipped => {}
        }
    }
    let verb = if dry_run { "would move" } else { "moved" };
    println!(
        "\n{moved} {verb}, {unknown} unknown{}",
        if failed > 0 { format!(", {failed} failed") } else { String::new() }
    );
    if failed > 0 { Err("some files could not be moved".into()) } else { Ok(()) }
}

fn cmd_stale(cfg: &Config, dir: PathBuf) {
    let rules = cfg.rules(&dir);
    let items = find_stale(cfg, &rules);
    if items.is_empty() {
        println!("Nothing in {} older than {} days. 🎉", tilde(&dir), cfg.stale.after_days);
        return;
    }
    let state = State::read();
    println!("{} item(s) in {} older than {} days:\n", items.len(), tilde(&dir), cfg.stale.after_days);
    for item in &items {
        let kept = if state.kept.contains(&item.path) { "  (kept)" } else { "" };
        println!("  {:>9}  {}{kept}", format_age(item.age_secs), stale::display_name(&rules, item));
    }
    println!("\nRun `file-sorter -r` to trash, move or keep them.");
}

fn cmd_review(cfg: &Config, dir: PathBuf) {
    let rules = cfg.rules(&dir);
    let state = State::read();
    let items: Vec<_> = find_stale(cfg, &rules).into_iter().filter(|i| !state.kept.contains(&i.path)).collect();
    if items.is_empty() {
        println!("Nothing to review in {} — no files older than {} days.", tilde(&dir), cfg.stale.after_days);
        return;
    }
    let total = items.len();
    let stdin = std::io::stdin();
    'items: for (n, item) in items.iter().enumerate() {
        let move_to = cfg.move_out_for(item.category.as_deref());
        println!("\n[{}/{total}] {}  ({})", n + 1, stale::display_name(&rules, item), format_age(item.age_secs));
        loop {
            let move_opt = move_to.as_ref().map(|d| format!("(m)ove to {}  ", tilde(d))).unwrap_or_default();
            print!("  (t)rash  {move_opt}(k)eep  (s)kip  (q)uit > ");
            let _ = std::io::stdout().flush();
            let mut line = String::new();
            if stdin.read_line(&mut line).unwrap_or(0) == 0 {
                break 'items; // EOF
            }
            let action = match line.trim().to_lowercase().as_str() {
                "t" => Action::Trash,
                "m" if move_to.is_some() => Action::MoveOut,
                "k" => Action::Keep,
                "s" | "" => continue 'items,
                "q" => break 'items,
                _ => continue,
            };
            match stale::apply(cfg, item, action) {
                Ok(msg) => println!("  ✓ {msg}"),
                Err(msg) => println!("  ✗ {msg}"),
            }
            continue 'items;
        }
    }
    println!("\nDone.");
}

fn cmd_rules(cfg: &Config) {
    let base = cfg.watch_dir();
    let rules = cfg.rules(&base);
    println!("Watching {}\n", tilde(&base));
    for (folder, exts) in &cfg.categories {
        let dest = rules.dest_for(folder);
        let shown = dest.strip_prefix(&base).map(|p| format!("{}/", p.display())).unwrap_or_else(|_| tilde(&dest));
        println!("  {shown:<14} ← {}", exts.join(" "));
    }
    println!("\nIgnored (in-progress downloads): {}", cfg.ignore_extensions.join(" "));
    let s = &cfg.stale;
    if s.enabled {
        println!(
            "\nReminders: files older than {} days, re-reminded every {} h, checked every {} min",
            s.after_days, s.remind_every_hours, s.check_every_minutes
        );
        println!("Move out targets:");
        for (folder, dest) in &cfg.move_out {
            println!("  {:<14} → {dest}", folder);
        }
    } else {
        println!("\nReminders: disabled");
    }
}
