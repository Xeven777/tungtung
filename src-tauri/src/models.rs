use serde::{Deserialize, Serialize};

/// A reminder row. Recurring reminders are materialized: the row always holds
/// the *next* due timestamp, and recurrence cycles it forward as each
/// occurrence fires (see `scheduler` + `recurrence`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: String,
    pub title: String,
    pub notes: Option<String>,
    /// RFC 3339 timestamp in UTC.
    pub due_at: String,
    pub timezone: Option<String>,
    /// Simplified RRULE, e.g. `FREQ=DAILY` or `FREQ=WEEKLY;BYDAY=MO,WE`.
    pub recurrence_rule: Option<String>,
    pub status: String,
    pub sound_id: Option<String>,
    pub notification_enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

/// Payload accepted when creating or updating a reminder from the UI.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderInput {
    pub title: String,
    pub notes: Option<String>,
    pub due_at: String,
    pub timezone: Option<String>,
    pub recurrence_rule: Option<String>,
    pub sound_id: Option<String>,
    #[serde(default = "default_true")]
    pub notification_enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Filter used by the Reminders page.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReminderFilter {
    /// `active` | `completed` | `overdue` | `all`
    pub scope: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Habit {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    /// `daily` | `weekdays` | `custom` | `times_per_week`
    pub schedule_type: String,
    /// JSON-encoded schedule payload (see `habits` docs).
    pub schedule_data: String,
    pub reminder_time: Option<String>,
    pub created_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HabitInput {
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub schedule_type: String,
    pub schedule_data: String,
    pub reminder_time: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HabitWithStats {
    #[serde(flatten)]
    pub habit: Habit,
    pub completed_this_week: i64,
    pub target_this_week: i64,
    pub current_streak: i64,
    pub best_streak: i64,
    pub completed_today: bool,
    pub week_dates: Vec<String>,
    pub week_completed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusSession {
    pub id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub planned_seconds: i64,
    pub actual_seconds: Option<i64>,
    /// `focus` | `short_break` | `long_break`
    pub session_type: String,
    pub completed: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusSessionInput {
    pub planned_seconds: i64,
    pub session_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sound {
    pub id: String,
    pub name: String,
    pub file_path: Option<String>,
    pub bundled: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingEntry {
    pub key: String,
    pub value: String,
}

/// Runtime environment diagnostics surfaced on the dev diagnostics page.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub platform: String,
    pub desktop: String,
    pub display: String,
    pub app_version: String,
    pub notification_available: bool,
    pub tray_available: bool,
    pub audio_available: bool,
    /// First installed backend sound player (`pw-play`, `paplay`, `ffplay`,
    /// ...), if any. When `None`, sounds can only play while the app window is
    /// open, and the UI should say what to install.
    pub audio_player: Option<String>,
    pub db_path: String,
}

/// Live micro-break timer state, shown in Settings so the interval is visible
/// rather than a hidden background behaviour.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicroBreakStatus {
    pub enabled: bool,
    pub remaining_seconds: Option<i64>,
}

/// Aggregated payload rendered by the Today screen.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayView {
    pub reminders: Vec<Reminder>,
    pub overdue: Vec<Reminder>,
    pub habits: Vec<HabitWithStats>,
    pub focus_seconds: i64,
}

/// Export envelope used by the Data settings section.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportBundle {
    pub version: u32,
    pub exported_at: String,
    pub reminders: Vec<Reminder>,
    pub habits: Vec<Habit>,
    pub habit_completions: Vec<HabitCompletion>,
    pub focus_sessions: Vec<FocusSession>,
    pub settings: Vec<SettingEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HabitCompletion {
    pub id: String,
    pub habit_id: String,
    pub completed_date: String,
    pub completed_at: String,
}

/// History rows merged across entity types.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub at: String,
    pub detail: Option<String>,
}
