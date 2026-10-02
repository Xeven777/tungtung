use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{Duration, Local, SecondsFormat, TimeZone, Utc};
use rusqlite::Connection;
use serde_json::Value;
use tauri::State;

use crate::db::{self, now_iso, today_local};
use crate::error::{AppError, AppResult};
use crate::microbreak::MicroBreak;
use crate::models::*;
use crate::platform;
use crate::prefs::RuntimePrefs;
use crate::scheduler::SchedulerHandle;

const ALLOWED_SOUND_EXTENSIONS: &[&str] = &["wav", "ogg", "mp3", "flac", "m4a", "aac", "opus"];

pub struct AppState {
    pub db: Mutex<Connection>,
    pub db_path: String,
    pub data_dir: String,
    pub scheduler: SchedulerHandle,
    pub prefs: Arc<RuntimePrefs>,
    pub micro_break: Arc<MicroBreak>,
}

impl AppState {
    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.db.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Nudge the scheduler after any write so it recomputes its next wake-up.
fn touch(state: &State<'_, AppState>) {
    state.scheduler.wake();
}

// ---------------------------------------------------------------------------
// Reminders
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_reminders(state: State<'_, AppState>, filter: Option<ReminderFilter>) -> AppResult<Vec<Reminder>> {
    db::list_reminders(&state.conn(), &filter.unwrap_or_default())
}

#[tauri::command]
pub fn create_reminder(state: State<'_, AppState>, input: ReminderInput) -> AppResult<Reminder> {
    let reminder = db::insert_reminder(&state.conn(), &input)?;
    touch(&state);
    Ok(reminder)
}

#[tauri::command]
pub fn update_reminder(state: State<'_, AppState>, id: String, input: ReminderInput) -> AppResult<Reminder> {
    let reminder = db::update_reminder(&state.conn(), &id, &input)?;
    touch(&state);
    Ok(reminder)
}

#[tauri::command]
pub fn complete_reminder(state: State<'_, AppState>, id: String) -> AppResult<Option<Reminder>> {
    let r = db::set_status(&state.conn(), &id, "completed", Some(now_iso()))?;
    touch(&state);
    Ok(r)
}

#[tauri::command]
pub fn reopen_reminder(state: State<'_, AppState>, id: String) -> AppResult<Option<Reminder>> {
    let r = db::set_status(&state.conn(), &id, "scheduled", None)?;
    touch(&state);
    Ok(r)
}

#[tauri::command]
pub fn skip_reminder(state: State<'_, AppState>, id: String) -> AppResult<Option<Reminder>> {
    let r = db::set_status(&state.conn(), &id, "skipped", Some(now_iso()))?;
    touch(&state);
    Ok(r)
}

#[tauri::command]
pub fn snooze_reminder(state: State<'_, AppState>, id: String, minutes: i64) -> AppResult<Option<Reminder>> {
    let due = Utc::now() + Duration::minutes(minutes);
    let due_str = due.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let conn = state.conn();
    conn.execute(
        "UPDATE reminders SET status='snoozed', due_at=?2, updated_at=?3 WHERE id=?1",
        rusqlite::params![id, due_str, now_iso()],
    )?;
    let r = db::get_reminder(&conn, &id)?;
    drop(conn);
    touch(&state);
    Ok(r)
}

#[tauri::command]
pub fn delete_reminder(state: State<'_, AppState>, id: String) -> AppResult<()> {
    db::delete_reminder(&state.conn(), &id)?;
    touch(&state);
    Ok(())
}

// ---------------------------------------------------------------------------
// Habits
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_habits(state: State<'_, AppState>) -> AppResult<Vec<HabitWithStats>> {
    db::habits_with_stats(&state.conn())
}

#[tauri::command]
pub fn create_habit(state: State<'_, AppState>, input: HabitInput) -> AppResult<Habit> {
    db::insert_habit(&state.conn(), &input)
}

#[tauri::command]
pub fn update_habit(state: State<'_, AppState>, id: String, input: HabitInput) -> AppResult<Habit> {
    db::update_habit(&state.conn(), &id, &input)
}

#[tauri::command]
pub fn archive_habit(state: State<'_, AppState>, id: String) -> AppResult<()> {
    db::archive_habit(&state.conn(), &id)
}

#[tauri::command]
pub fn toggle_habit(state: State<'_, AppState>, habit_id: String, date: Option<String>) -> AppResult<bool> {
    let date = date.unwrap_or_else(|| today_local().format("%Y-%m-%d").to_string());
    db::toggle_habit_completion(&state.conn(), &habit_id, &date)
}

// ---------------------------------------------------------------------------
// Focus sessions
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn start_focus_session(state: State<'_, AppState>, input: FocusSessionInput) -> AppResult<FocusSession> {
    db::start_focus_session(&state.conn(), &input)
}

#[tauri::command]
pub fn end_focus_session(
    state: State<'_, AppState>,
    id: String,
    actual_seconds: i64,
    completed: bool,
) -> AppResult<Option<FocusSession>> {
    db::end_focus_session(&state.conn(), &id, actual_seconds, completed)
}

#[tauri::command]
pub fn list_focus_sessions(state: State<'_, AppState>, limit: Option<i64>) -> AppResult<Vec<FocusSession>> {
    db::list_focus_sessions(&state.conn(), limit.unwrap_or(50))
}

// ---------------------------------------------------------------------------
// Sounds & settings
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_sounds(state: State<'_, AppState>) -> AppResult<Vec<Sound>> {
    db::list_sounds(&state.conn())
}

#[tauri::command]
pub fn add_sound(state: State<'_, AppState>, name: String, file_path: String) -> AppResult<Sound> {
    db::insert_sound(&state.conn(), &name, &file_path)
}

/// Copy a user-chosen audio file into the app data directory and register it.
/// Storing our own copy keeps imports self-contained and inside the asset
/// protocol scope (`$APPDATA/**`), so playback keeps working across restarts.
#[tauri::command]
pub fn import_sound(state: State<'_, AppState>, name: String, source_path: String) -> AppResult<Sound> {
    let source = PathBuf::from(&source_path);
    let extension = source
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .ok_or_else(|| AppError::msg("sound file has no extension"))?;

    if !ALLOWED_SOUND_EXTENSIONS.contains(&extension.as_str()) {
        return Err(AppError::msg(format!(
            "unsupported sound format .{extension} (try wav, ogg or mp3)"
        )));
    }

    let label = if name.trim().is_empty() {
        source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Custom sound")
            .to_string()
    } else {
        name.trim().to_string()
    };

    let sounds_dir = Path::new(&state.data_dir).join("sounds");
    std::fs::create_dir_all(&sounds_dir)?;

    let file_name = format!("{}.{extension}", uuid::Uuid::new_v4());
    let destination = sounds_dir.join(file_name);
    std::fs::copy(&source, &destination)?;

    let path_string = destination.to_string_lossy().to_string();
    db::insert_sound(&state.conn(), &label, &path_string)
}

#[tauri::command]
pub fn delete_sound(state: State<'_, AppState>, id: String) -> AppResult<()> {
    db::delete_sound(&state.conn(), &id)
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppResult<HashMap<String, String>> {
    db::all_settings(&state.conn())
}

#[tauri::command]
pub fn set_setting(state: State<'_, AppState>, key: String, value: String) -> AppResult<()> {
    db::put_setting(&state.conn(), &key, &value)?;
    // Keep the scheduler's copy in sync so gating changes take effect at once.
    state.prefs.apply(&key, &value);
    // Only a change to the break schedule itself should restart the interval.
    if key.starts_with("microBreak") || key == "quietHoursEnabled" {
        reset_micro_break(&state);
    }
    state.scheduler.wake();
    Ok(())
}

// ---------------------------------------------------------------------------
// Micro breaks
// ---------------------------------------------------------------------------

/// Restart the work interval from now. Called when a break ends, when the user
/// snoozes, and after any settings write that could change the schedule.
fn reset_micro_break(state: &State<'_, AppState>) {
    let prefs = state.prefs.snapshot();
    state.micro_break.reset(work_interval(&prefs), Local::now());
}

fn work_interval(prefs: &crate::prefs::Prefs) -> Duration {
    Duration::minutes(i64::from(prefs.micro_work_minutes.max(1)))
}

#[tauri::command]
pub fn micro_break_status(state: State<'_, AppState>) -> AppResult<MicroBreakStatus> {
    let prefs = state.prefs.snapshot();
    Ok(MicroBreakStatus {
        enabled: prefs.micro_breaks_enabled,
        remaining_seconds: if prefs.micro_breaks_enabled {
            state.micro_break.remaining(Local::now())
        } else {
            None
        },
    })
}

#[tauri::command]
pub fn micro_break_reset(state: State<'_, AppState>) -> AppResult<()> {
    reset_micro_break(&state);
    Ok(())
}

/// "Snooze" on the break notification: come back in `minutes` instead of
/// waiting out a whole work interval.
#[tauri::command]
pub fn micro_break_snooze(state: State<'_, AppState>, minutes: u32) -> AppResult<()> {
    let minutes = minutes.clamp(1, 120);
    state
        .micro_break
        .reset(Duration::minutes(i64::from(minutes)), Local::now());
    state.scheduler.wake();
    Ok(())
}

/// "Start break now" — used by the tray, the settings screen, and the overlay's
/// own snooze path so a break can be taken on demand.
#[tauri::command]
pub fn micro_break_trigger(state: State<'_, AppState>) -> AppResult<()> {
    state.micro_break.fire_now(Local::now());
    state.scheduler.wake();
    Ok(())
}

// ---------------------------------------------------------------------------
// Aggregates
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn today_view(state: State<'_, AppState>) -> AppResult<TodayView> {
    let conn = state.conn();
    let now = now_iso();
    let start = chrono::Local::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .and_then(|naive| chrono::Local.from_local_datetime(&naive).single())
        .map(|d| d.with_timezone(&Utc).to_rfc3339_opts(SecondsFormat::Secs, true))
        .unwrap_or_else(|| "0000-01-01T00:00:00Z".to_string());

    let due_end = format!("{}T23:59:59Z", today_local().format("%Y-%m-%d"));

    let all = db::list_reminders(&conn, &ReminderFilter { scope: Some("active".into()), search: None })?;
    let overdue: Vec<Reminder> = all
        .iter()
        .filter(|r| r.status == "overdue" || r.due_at.as_str() < now.as_str())
        .cloned()
        .collect();
    let reminders: Vec<Reminder> = all
        .into_iter()
        .filter(|r| r.due_at >= now && r.due_at <= due_end)
        .collect();

    let focus_seconds = db::focus_seconds_since(&conn, &start)?;

    Ok(TodayView {
        reminders,
        overdue,
        habits: db::habits_with_stats(&conn)?,
        focus_seconds,
    })
}

#[tauri::command]
pub fn history(state: State<'_, AppState>, days: Option<i64>) -> AppResult<Vec<HistoryEntry>> {
    let since = days
        .filter(|d| *d > 0)
        .map(|d| (Utc::now() - Duration::days(d)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    db::history(&state.conn(), since.as_deref())
}

#[tauri::command]
pub fn activity_counts(state: State<'_, AppState>, days: Option<i64>) -> AppResult<Vec<(String, i64)>> {
    db::activity_counts(&state.conn(), days.unwrap_or(14))
}

// ---------------------------------------------------------------------------
// System
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn diagnostics(app: tauri::AppHandle, state: State<'_, AppState>) -> AppResult<Diagnostics> {
    let version = app.package_info().version.to_string();
    Ok(platform::diagnostics(&version, &state.db_path))
}

#[tauri::command]
pub fn export_data(state: State<'_, AppState>) -> AppResult<ExportBundle> {
    let conn = state.conn();
    let reminders = db::list_reminders(&conn, &ReminderFilter { scope: Some("all".into()), search: None })?;
    let habits = all_habits(&conn)?;
    let habit_completions = all_completions(&conn)?;
    let focus_sessions = db::list_focus_sessions(&conn, 100_000)?;
    let settings = db::all_settings(&conn)?
        .into_iter()
        .map(|(key, value)| SettingEntry { key, value })
        .collect();

    Ok(ExportBundle {
        version: 1,
        exported_at: now_iso(),
        reminders,
        habits,
        habit_completions,
        focus_sessions,
        settings,
    })
}

#[tauri::command]
pub fn import_data(state: State<'_, AppState>, bundle: ExportBundle) -> AppResult<()> {
    let conn = state.conn();
    for reminder in &bundle.reminders {
        conn.execute(
            "INSERT OR REPLACE INTO reminders
             (id, title, notes, due_at, timezone, recurrence_rule, status, sound_id,
              notification_enabled, created_at, updated_at, completed_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            rusqlite::params![
                reminder.id, reminder.title, reminder.notes, reminder.due_at,
                reminder.timezone, reminder.recurrence_rule, reminder.status,
                reminder.sound_id, reminder.notification_enabled as i64,
                reminder.created_at, reminder.updated_at, reminder.completed_at
            ],
        )?;
    }
    for habit in &bundle.habits {
        conn.execute(
            "INSERT OR REPLACE INTO habits
             (id, name, icon, color, schedule_type, schedule_data, reminder_time, created_at, archived_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            rusqlite::params![
                habit.id, habit.name, habit.icon, habit.color, habit.schedule_type,
                habit.schedule_data, habit.reminder_time, habit.created_at, habit.archived_at
            ],
        )?;
    }
    for completion in &bundle.habit_completions {
        conn.execute(
            "INSERT OR IGNORE INTO habit_completions (id, habit_id, completed_date, completed_at)
             VALUES (?1,?2,?3,?4)",
            rusqlite::params![completion.id, completion.habit_id, completion.completed_date, completion.completed_at],
        )?;
    }
    for session in &bundle.focus_sessions {
        conn.execute(
            "INSERT OR REPLACE INTO focus_sessions
             (id, started_at, ended_at, planned_seconds, actual_seconds, type, completed)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            rusqlite::params![
                session.id, session.started_at, session.ended_at, session.planned_seconds,
                session.actual_seconds, session.session_type, session.completed as i64
            ],
        )?;
    }
    for entry in &bundle.settings {
        db::put_setting(&conn, &entry.key, &entry.value)?;
    }

    drop(conn);
    touch(&state);
    Ok(())
}

/// Wipe every user-created record.
///
/// Settings and bundled sounds survive: they are configuration the app needs to
/// keep working, not content. Custom sound rows go too, but their files are left
/// on disk — deleting a user's audio is not something a data wipe should do
/// behind their back.
#[tauri::command]
pub fn clear_all_data(state: State<'_, AppState>) -> AppResult<()> {
    let conn = state.conn();
    conn.execute_batch(
        "DELETE FROM habit_completions;
         DELETE FROM habit_reminder_log;
         DELETE FROM reminders;
         DELETE FROM habits;
         DELETE FROM focus_sessions;
         DELETE FROM sounds WHERE bundled = 0;",
    )?;
    drop(conn);
    touch(&state);
    Ok(())
}

/// Validate a raw JSON export payload before importing.
#[tauri::command]
pub fn parse_export(raw: String) -> AppResult<ExportBundle> {
    let value: Value = serde_json::from_str(&raw)?;
    serde_json::from_value(value).map_err(|e| AppError::msg(format!("invalid export file: {e}")))
}

fn all_habits(conn: &Connection) -> AppResult<Vec<Habit>> {
    let mut stmt = conn.prepare("SELECT * FROM habits")?;
    let rows = stmt.query_map([], |row| {
        Ok(Habit {
            id: row.get("id")?,
            name: row.get("name")?,
            icon: row.get("icon")?,
            color: row.get("color")?,
            schedule_type: row.get("schedule_type")?,
            schedule_data: row.get("schedule_data")?,
            reminder_time: row.get("reminder_time")?,
            created_at: row.get("created_at")?,
            archived_at: row.get("archived_at")?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

fn all_completions(conn: &Connection) -> AppResult<Vec<HabitCompletion>> {
    let mut stmt = conn.prepare("SELECT id, habit_id, completed_date, completed_at FROM habit_completions")?;
    let rows = stmt.query_map([], |row| {
        Ok(HabitCompletion {
            id: row.get(0)?,
            habit_id: row.get(1)?,
            completed_date: row.get(2)?,
            completed_at: row.get(3)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}
