use std::collections::HashMap;
use std::sync::Mutex;

use chrono::{Local, NaiveTime, Timelike};
use rusqlite::Connection;

use crate::db;

/// The subset of settings that the Rust side must act on. These are read at
/// startup and refreshed on every `set_setting`, so the scheduler never has to
/// query SQLite on the hot path.
#[derive(Debug, Clone)]
pub struct Prefs {
    /// Global notification gate (`notificationsEnabled`).
    pub notifications_enabled: bool,
    pub quiet_hours_enabled: bool,
    pub quiet_start: String,
    pub quiet_end: String,
    /// Keep running in the tray when the window is closed.
    pub close_to_tray: bool,
    /// Start hidden instead of showing the window.
    pub start_minimized: bool,
    /// Gentle "look away from the screen" nudges.
    pub micro_breaks_enabled: bool,
    pub micro_work_minutes: u32,
    pub micro_break_minutes: u32,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            notifications_enabled: true,
            quiet_hours_enabled: false,
            quiet_start: "23:00".into(),
            quiet_end: "08:00".into(),
            close_to_tray: true,
            start_minimized: false,
            micro_breaks_enabled: true,
            micro_work_minutes: 50,
            micro_break_minutes: 5,
        }
    }
}

impl Prefs {
    /// Quiet hours may wrap past midnight (23:00 → 08:00).
    pub fn in_quiet_hours(&self, now: chrono::DateTime<Local>) -> bool {
        if !self.quiet_hours_enabled {
            return false;
        }
        let (Some(start), Some(end)) = (parse_hhmm(&self.quiet_start), parse_hhmm(&self.quiet_end))
        else {
            return false;
        };
        let minutes = now.hour() * 60 + now.minute();
        let start = start.hour() * 60 + start.minute();
        let end = end.hour() * 60 + end.minute();
        if start == end {
            return false;
        }
        if start < end {
            minutes >= start && minutes < end
        } else {
            minutes >= start || minutes < end
        }
    }

    /// True when an OS notification should be suppressed right now.
    pub fn notifications_suppressed(&self) -> bool {
        !self.notifications_enabled || self.in_quiet_hours(Local::now())
    }
}

fn parse_hhmm(value: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(value.trim(), "%H:%M").ok()
}

fn truthy(value: &str) -> bool {
    value.eq_ignore_ascii_case("true") || value == "1"
}

/// Minutes are clamped so a stray value can never spin the scheduler or park it
/// for a day. Returns `None` when the value is not a usable number.
fn parse_minutes(value: &str) -> Option<u32> {
    let minutes = value.trim().parse::<u32>().ok()?;
    Some(minutes.clamp(1, 600))
}

fn minutes(settings: &HashMap<String, String>, key: &str, fallback: u32) -> u32 {
    settings
        .get(key)
        .and_then(|value| parse_minutes(value))
        .unwrap_or(fallback)
}

/// Thread-safe holder shared between the Tauri commands and the scheduler.
pub struct RuntimePrefs(Mutex<Prefs>);

impl RuntimePrefs {
    pub fn load(conn: &Connection) -> Self {
        let mut prefs = Prefs::default();
        if let Ok(settings) = db::all_settings(conn) {
            prefs.notifications_enabled = settings
                .get("notificationsEnabled")
                .map(|v| truthy(v))
                .unwrap_or(true);
            prefs.quiet_hours_enabled = settings
                .get("quietHoursEnabled")
                .map(|v| truthy(v))
                .unwrap_or(false);
            if let Some(value) = settings.get("quietStart") {
                prefs.quiet_start = value.clone();
            }
            if let Some(value) = settings.get("quietEnd") {
                prefs.quiet_end = value.clone();
            }
            prefs.close_to_tray = settings.get("closeToTray").map(|v| truthy(v)).unwrap_or(true);
            prefs.start_minimized = settings
                .get("startMinimized")
                .map(|v| truthy(v))
                .unwrap_or(false);
            prefs.micro_breaks_enabled = settings
                .get("microBreaksEnabled")
                .map(|v| truthy(v))
                .unwrap_or(true);
            prefs.micro_work_minutes =
                minutes(&settings, "microWorkMinutes", prefs.micro_work_minutes);
            prefs.micro_break_minutes =
                minutes(&settings, "microBreakMinutes", prefs.micro_break_minutes);
        }
        RuntimePrefs(Mutex::new(prefs))
    }

    pub fn snapshot(&self) -> Prefs {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Apply a single settings write. Unknown keys are ignored.
    pub fn apply(&self, key: &str, value: &str) {
        let mut prefs = self.0.lock().unwrap_or_else(|e| e.into_inner());
        match key {
            "notificationsEnabled" => prefs.notifications_enabled = truthy(value),
            "quietHoursEnabled" => prefs.quiet_hours_enabled = truthy(value),
            "quietStart" => prefs.quiet_start = value.to_string(),
            "quietEnd" => prefs.quiet_end = value.to_string(),
            "closeToTray" => prefs.close_to_tray = truthy(value),
            "startMinimized" => prefs.start_minimized = truthy(value),
            "microBreaksEnabled" => prefs.micro_breaks_enabled = truthy(value),
            "microWorkMinutes" => {
                if let Some(m) = parse_minutes(value) {
                    prefs.micro_work_minutes = m;
                }
            }
            "microBreakMinutes" => {
                if let Some(m) = parse_minutes(value) {
                    prefs.micro_break_minutes = m;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(hour: u32, minute: u32) -> chrono::DateTime<Local> {
        Local
            .with_ymd_and_hms(2026, 10, 2, hour, minute, 0)
            .single()
            .unwrap()
    }

    #[test]
    fn quiet_hours_wrap_past_midnight() {
        let prefs = Prefs {
            quiet_hours_enabled: true,
            quiet_start: "23:00".into(),
            quiet_end: "08:00".into(),
            ..Prefs::default()
        };
        assert!(prefs.in_quiet_hours(at(23, 30)));
        assert!(prefs.in_quiet_hours(at(2, 0)));
        assert!(prefs.in_quiet_hours(at(7, 59)));
        assert!(!prefs.in_quiet_hours(at(8, 0)));
        assert!(!prefs.in_quiet_hours(at(12, 0)));
    }

    #[test]
    fn quiet_hours_disabled_is_never_quiet() {
        let prefs = Prefs {
            quiet_hours_enabled: false,
            quiet_start: "23:00".into(),
            quiet_end: "08:00".into(),
            ..Prefs::default()
        };
        assert!(!prefs.in_quiet_hours(at(2, 0)));
    }

    #[test]
    fn global_toggle_suppresses_notifications() {
        let prefs = Prefs { notifications_enabled: false, ..Prefs::default() };
        assert!(prefs.notifications_suppressed());
    }
}
