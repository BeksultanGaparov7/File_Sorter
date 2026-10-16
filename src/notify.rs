// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Merlin Hermes

use notify_rust::{Notification, Urgency};

fn build(summary: &str, body: &str, icon: &str, urgency: Urgency) -> Notification {
    let mut n = Notification::new();
    n.appname("File Sorter").summary(summary).body(&escape(body)).icon(icon).urgency(urgency);
    n
}

pub fn send(summary: &str, body: &str, icon: &str, urgency: Urgency) {
    if let Err(e) = build(summary, body, icon, urgency).show() {
        eprintln!("WARN notification failed: {e}");
    }
}

/// Show a notification with buttons; `on_action` runs on a background thread
/// with the chosen action id ("default" = notification clicked, "__closed" = dismissed).
pub fn send_with_actions(
    summary: &str,
    body: &str,
    icon: &str,
    actions: &[(&str, String)],
    on_action: impl FnOnce(&str) + Send + 'static,
) {
    let mut n = build(summary, body, icon, Urgency::Normal);
    n.action("default", "Open folder");
    for (id, label) in actions {
        n.action(id, label);
    }
    match n.show() {
        Ok(handle) => {
            std::thread::spawn(move || handle.wait_for_action(on_action));
        }
        Err(e) => eprintln!("WARN notification failed: {e}"),
    }
}

/// The notification daemon renders body markup, so file names must be escaped.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
