//! Pretty system notifications for Linux (XFCE and friends).
//!
//! Notifications go through `notify-rust` directly so we can send a
//! fully-dressed `org.freedesktop.Notifications` payload: app name + icon,
//! urgency, timeout, and category / sound hints. (The old Tauri notification
//! plugin only forwarded title / body / icon / sound, which is why XFCE
//! bubbles looked plain.)
//!
//! Each desktop environment renders the bubble in its own theme — XFCE,
//! GNOME and KDE all respect the same hints but draw them differently.

use std::path::PathBuf;

use notify_rust::{Hint, Notification, Timeout, Urgency};

/// How "loud" a notification should feel to the notification daemon.
#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    /// A due reminder — must stand out in xfce4-notifyd.
    Reminder,
    /// Habits and breaks — friendly, still noticeable.
    Nudge,
}

impl Kind {
    fn urgency(self) -> Urgency {
        match self {
            Kind::Reminder => Urgency::Critical,
            Kind::Nudge => Urgency::Normal,
        }
    }

    fn timeout(self) -> Timeout {
        match self {
            // Critical urgency already keeps it on screen in xfce4-notifyd;
            // 30s is a safety net for daemons that expire critical anyway.
            Kind::Reminder => Timeout::Milliseconds(30_000),
            Kind::Nudge => Timeout::Milliseconds(12_000),
        }
    }

    fn category(self) -> &'static str {
        match self {
            Kind::Reminder => "alarm",
            Kind::Nudge => "reminder",
        }
    }
}

/// Look for the bundled app icon next to the running binary / resources so the
/// bubble shows our logo instead of a generic bell.
fn app_icon() -> Option<String> {
    let names = ["icon.png", "128x128.png", "32x32.png"];

    // Dev layout: `<repo>/src-tauri/icons/...`
    let mut candidates: Vec<PathBuf> = names
        .iter()
        .map(|n| PathBuf::from(format!("src-tauri/icons/{n}")))
        .collect();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for n in &names {
                // Installed layout: resources dir next to the binary.
                candidates.push(dir.join(n));
            }
            candidates.push(dir.join(
                "../share/icons/hicolor/128x128/apps/com.anish.tungtung.png",
            ));
            // Tauri resource dir when bundled.
            candidates.push(dir.join("icon.png"));
        }
    }

    // Hicolor theme locations (work when the .deb installed the icon).
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        candidates.push(home.join(".local/share/icons/hicolor/128x128/apps/com.anish.tungtung.png"));
    }
    candidates.push(PathBuf::from(
        "/usr/share/icons/hicolor/128x128/apps/com.anish.tungtung.png",
    ));

    candidates.into_iter().find_map(|p| {
        if p.is_file() {
            p.to_str().map(|s| s.to_owned())
        } else {
            None
        }
    })
}

/// Send a notification. Failures are logged by the caller; this never panics.
pub fn send(title: &str, body: &str, kind: Kind, sound: Option<&str>) {
    let mut n = Notification::new();
    n.appname("TungTung");
    n.summary(title);
    n.body(body);
    n.urgency(kind.urgency());
    n.timeout(kind.timeout());
    n.hint(Hint::Category(kind.category().to_owned()));
    n.hint(Hint::Transient(false));
    n.hint(Hint::Resident(kind == Kind::Reminder));

    if let Some(icon) = app_icon() {
        n.icon(&icon);
        // Also expose the file image; some daemons prefer image-path.
        n.image_path(&icon);
    } else {
        n.icon("appointment-soon");
    }

    if let Some(sound) = sound {
        let sound = sound.trim();
        if !sound.is_empty() && sound != "none" {
            // XDG theme sound name (e.g. "message-new-instant").
            n.sound_name(sound);
            n.hint(Hint::SoundName(sound.to_owned()));
        }
    }

    if let Err(err) = n.show() {
        eprintln!("notify: failed to show notification: {err}");
    }
}

/// Map the app's stored `sound_id` to an XDG theme sound, if any.
pub fn theme_sound(sound_id: Option<&str>) -> Option<&'static str> {
    match sound_id {
        Some("builtin-bell") => Some("complete"),
        Some("builtin-minimal") => Some("message-new-instant"),
        Some("builtin-soft") => Some("message"),
        Some(other) if !other.is_empty() && other != "none" => Some("message-new-instant"),
        _ => None,
    }
}
