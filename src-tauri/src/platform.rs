use crate::models::Diagnostics;

fn env_first(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
}

fn detect_desktop() -> String {
    if let Some(desktop) = env_first(&["XDG_CURRENT_DESKTOP", "DESKTOP_SESSION"]) {
        // Values like "ubuntu:GNOME" or "KDE" -> keep the most specific segment.
        let primary = desktop
            .split(':')
            .last()
            .unwrap_or(&desktop)
            .trim()
            .to_string();
        return if primary.is_empty() { "Unknown".into() } else { primary };
    }
    "Unknown".into()
}

fn detect_display() -> String {
    if std::env::var("WAYLAND_DISPLAY").map(|v| !v.is_empty()).unwrap_or(false) {
        "Wayland".into()
    } else if std::env::var("DISPLAY").map(|v| !v.is_empty()).unwrap_or(false) {
        "X11".into()
    } else {
        "Headless".into()
    }
}

pub fn diagnostics(app_version: &str, db_path: &str) -> Diagnostics {
    Diagnostics {
        platform: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        desktop: detect_desktop(),
        display: detect_display(),
        app_version: app_version.to_string(),
        notification_available: cfg!(target_os = "linux") || cfg!(target_os = "macos") || cfg!(target_os = "windows"),
        tray_available: true,
        audio_available: cfg!(target_os = "linux"),
        audio_player: crate::sound::available_player().map(|name| name.to_string()),
        db_path: db_path.to_string(),
    }
}
