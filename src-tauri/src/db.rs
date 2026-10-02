use std::collections::HashMap;
use std::path::Path;

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::AppResult;
use crate::models::*;

pub fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn today_local() -> NaiveDate {
    Local::now().date_naive()
}

/// Open (creating if needed) the SQLite database and run migrations.
pub fn open(path: &Path) -> AppResult<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    migrate(&conn)?;
    Ok(conn)
}

fn user_version(conn: &Connection) -> AppResult<i64> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}

fn set_user_version(conn: &Connection, version: i64) -> AppResult<()> {
    conn.execute_batch(&format!("PRAGMA user_version = {version}"))?;
    Ok(())
}

/// Sequential, forward-only migrations keyed off `PRAGMA user_version`.
pub fn migrate(conn: &Connection) -> AppResult<()> {
    let current = user_version(conn)?;

    if current < 1 {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS reminders (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                notes TEXT,
                due_at TEXT NOT NULL,
                timezone TEXT,
                recurrence_rule TEXT,
                status TEXT NOT NULL DEFAULT 'scheduled',
                sound_id TEXT,
                notification_enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                completed_at TEXT
            );

            CREATE TABLE IF NOT EXISTS habits (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                icon TEXT,
                color TEXT,
                schedule_type TEXT NOT NULL,
                schedule_data TEXT NOT NULL,
                reminder_time TEXT,
                created_at TEXT NOT NULL,
                archived_at TEXT
            );

            CREATE TABLE IF NOT EXISTS habit_completions (
                id TEXT PRIMARY KEY,
                habit_id TEXT NOT NULL,
                completed_date TEXT NOT NULL,
                completed_at TEXT NOT NULL,
                UNIQUE(habit_id, completed_date)
            );

            CREATE TABLE IF NOT EXISTS focus_sessions (
                id TEXT PRIMARY KEY,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                planned_seconds INTEGER NOT NULL,
                actual_seconds INTEGER,
                type TEXT NOT NULL,
                completed INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS sounds (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                file_path TEXT,
                bundled INTEGER NOT NULL DEFAULT 0,
                enabled INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_reminders_status_due
                ON reminders(status, due_at);
            CREATE INDEX IF NOT EXISTS idx_habit_completions_habit
                ON habit_completions(habit_id, completed_date);
            CREATE INDEX IF NOT EXISTS idx_focus_sessions_started
                ON focus_sessions(started_at);
            "#,
        )?;
        set_user_version(conn, 1)?;
    }

    if current < 2 {
        // Tracks which habit reminders have already fired on a given local
        // date, so restarting the app never double-notifies.
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS habit_reminder_log (
                habit_id TEXT NOT NULL,
                fire_date TEXT NOT NULL,
                fired_at TEXT NOT NULL,
                PRIMARY KEY (habit_id, fire_date)
            );
            "#,
        )?;
        set_user_version(conn, 2)?;
    }

    seed_bundled_sounds(conn)?;
    Ok(())
}

const BUNDLED_SYNTH_SOUNDS: &[&str] = &[
    "Soft", "Classic", "Bell", "Wood", "Digital", "Minimal",
];

/// Meme clips shipped with the app. The frontend plays them via Vite-emitted
/// asset URLs (`BUNDLED_CLIP_URLS` in `sounds.ts`); `file_path` is a fallback
/// hint for older DB rows, not the load path.
/// `bundled = 2` distinguishes file-backed defaults from synth defaults
/// (`bundled = 1`) so `delete_sound` (which only allows `bundled = 0`) still
/// protects both, and `clear_all_data` keeps both.
const BUNDLED_FILE_SOUNDS: &[(&str, &str)] = &[
    ("Tung Tung", "tungtung.opus"),
    ("Siren", "siren.opus"),
    ("Scream", "screeam.opus"),
    ("Anime Ahh", "anime-ahh.opus"),
    ("Faaah", "faaah.opus"),
    ("Cat Laugh", "cat-laugh.opus"),
];

fn seed_bundled_sounds(conn: &Connection) -> AppResult<()> {
    for name in BUNDLED_SYNTH_SOUNDS {
        let id = format!("builtin-{}", name.to_lowercase());
        conn.execute(
            "INSERT OR IGNORE INTO sounds (id, name, file_path, bundled, enabled)
             VALUES (?1, ?2, NULL, 1, 1)",
            params![id, name],
        )?;
    }
    for (name, file) in BUNDLED_FILE_SOUNDS {
        let id = format!("bundled-{}", file.trim_end_matches(".opus"));
        // `file_path` is NULL on purpose: bundled clips are served from the
        // webview bundle via BUNDLED_CLIP_URLS, not from disk. Seeding a fake
        // path here only creates a landmine if that map ever loses an entry.
        conn.execute(
            "INSERT OR IGNORE INTO sounds (id, name, file_path, bundled, enabled)
             VALUES (?1, ?2, NULL, 2, 1)",
            params![id, name],
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Reminders
// ---------------------------------------------------------------------------

fn row_to_reminder(row: &rusqlite::Row) -> rusqlite::Result<Reminder> {
    Ok(Reminder {
        id: row.get("id")?,
        title: row.get("title")?,
        notes: row.get("notes")?,
        due_at: row.get("due_at")?,
        timezone: row.get("timezone")?,
        recurrence_rule: row.get("recurrence_rule")?,
        status: row.get("status")?,
        sound_id: row.get("sound_id")?,
        notification_enabled: row.get::<_, i64>("notification_enabled")? != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        completed_at: row.get("completed_at")?,
    })
}

pub fn list_reminders(conn: &Connection, filter: &ReminderFilter) -> AppResult<Vec<Reminder>> {
    let mut sql = String::from("SELECT * FROM reminders");
    let mut clauses: Vec<String> = Vec::new();

    match filter.scope.as_deref() {
        Some("active") => clauses.push("status IN ('scheduled','snoozed','overdue')".into()),
        Some("completed") => clauses.push("status = 'completed'".into()),
        Some("overdue") => clauses.push("status = 'overdue'".into()),
        _ => {}
    }
    if let Some(search) = filter.search.as_ref().filter(|s| !s.trim().is_empty()) {
        let escaped = search.replace('\'', "''");
        clauses.push(format!(
            "(title LIKE '%{escaped}%' OR IFNULL(notes,'') LIKE '%{escaped}%')"
        ));
    }
    if !clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&clauses.join(" AND "));
    }
    sql.push_str(" ORDER BY due_at ASC");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], row_to_reminder)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn get_reminder(conn: &Connection, id: &str) -> AppResult<Option<Reminder>> {
    let r = conn
        .query_row("SELECT * FROM reminders WHERE id = ?1", params![id], row_to_reminder)
        .optional()?;
    Ok(r)
}

pub fn insert_reminder(conn: &Connection, input: &ReminderInput) -> AppResult<Reminder> {
    let now = now_iso();
    let reminder = Reminder {
        id: uuid::Uuid::new_v4().to_string(),
        title: input.title.trim().to_string(),
        notes: input.notes.clone(),
        due_at: input.due_at.clone(),
        timezone: input.timezone.clone(),
        recurrence_rule: input.recurrence_rule.clone(),
        status: "scheduled".into(),
        sound_id: input.sound_id.clone(),
        notification_enabled: input.notification_enabled,
        created_at: now.clone(),
        updated_at: now,
        completed_at: None,
    };

    if reminder.title.is_empty() {
        return Err(crate::error::AppError::msg("title is required"));
    }
    DateTime::parse_from_rfc3339(&reminder.due_at)
        .map_err(|_| crate::error::AppError::msg("dueAt must be an RFC 3339 timestamp"))?;

    conn.execute(
        "INSERT INTO reminders
         (id, title, notes, due_at, timezone, recurrence_rule, status, sound_id,
          notification_enabled, created_at, updated_at, completed_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            reminder.id,
            reminder.title,
            reminder.notes,
            reminder.due_at,
            reminder.timezone,
            reminder.recurrence_rule,
            reminder.status,
            reminder.sound_id,
            reminder.notification_enabled as i64,
            reminder.created_at,
            reminder.updated_at,
            reminder.completed_at,
        ],
    )?;
    Ok(reminder)
}

pub fn update_reminder(conn: &Connection, id: &str, input: &ReminderInput) -> AppResult<Reminder> {
    let existing = get_reminder(conn, id)?.ok_or_else(|| crate::error::AppError::msg("reminder not found"))?;
    let now = now_iso();
    conn.execute(
        "UPDATE reminders SET title=?2, notes=?3, due_at=?4, timezone=?5,
         recurrence_rule=?6, sound_id=?7, notification_enabled=?8, updated_at=?9
         WHERE id=?1",
        params![
            id,
            input.title.trim(),
            input.notes,
            input.due_at,
            input.timezone,
            input.recurrence_rule,
            input.sound_id,
            input.notification_enabled as i64,
            now,
        ],
    )?;
    let mut updated = get_reminder(conn, id)?.unwrap_or(existing);
    updated.updated_at = now;
    Ok(updated)
}

pub fn set_status(conn: &Connection, id: &str, status: &str, completed_at: Option<String>) -> AppResult<Option<Reminder>> {
    conn.execute(
        "UPDATE reminders SET status=?2, completed_at=?3, updated_at=?4 WHERE id=?1",
        params![id, status, completed_at, now_iso()],
    )?;
    get_reminder(conn, id)
}

pub fn delete_reminder(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM reminders WHERE id=?1", params![id])?;
    Ok(())
}

/// All scheduled reminders whose next occurrence is due at or before `before`.
pub fn due_reminders(conn: &Connection, before: &str) -> AppResult<Vec<Reminder>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM reminders
         WHERE status IN ('scheduled','snoozed') AND due_at <= ?1
         ORDER BY due_at ASC",
    )?;
    let rows = stmt.query_map(params![before], row_to_reminder)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Earliest due timestamp across active reminders, used to size the scheduler sleep.
pub fn next_due_at(conn: &Connection) -> AppResult<Option<String>> {
    let v = conn
        .query_row(
            "SELECT MIN(due_at) FROM reminders WHERE status IN ('scheduled','snoozed')",
            [],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten();
    Ok(v)
}

// ---------------------------------------------------------------------------
// Habits
// ---------------------------------------------------------------------------

fn row_to_habit(row: &rusqlite::Row) -> rusqlite::Result<Habit> {
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
}

pub fn list_habits(conn: &Connection) -> AppResult<Vec<Habit>> {
    let mut stmt = conn.prepare("SELECT * FROM habits WHERE archived_at IS NULL ORDER BY created_at ASC")?;
    let rows = stmt.query_map([], row_to_habit)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn insert_habit(conn: &Connection, input: &HabitInput) -> AppResult<Habit> {
    if input.name.trim().is_empty() {
        return Err(crate::error::AppError::msg("habit name is required"));
    }
    let habit = Habit {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name.trim().to_string(),
        icon: input.icon.clone(),
        color: input.color.clone(),
        schedule_type: input.schedule_type.clone(),
        schedule_data: input.schedule_data.clone(),
        reminder_time: input.reminder_time.clone(),
        created_at: now_iso(),
        archived_at: None,
    };
    conn.execute(
        "INSERT INTO habits (id, name, icon, color, schedule_type, schedule_data, reminder_time, created_at, archived_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,NULL)",
        params![
            habit.id, habit.name, habit.icon, habit.color,
            habit.schedule_type, habit.schedule_data, habit.reminder_time, habit.created_at
        ],
    )?;
    Ok(habit)
}

pub fn update_habit(conn: &Connection, id: &str, input: &HabitInput) -> AppResult<Habit> {
    conn.execute(
        "UPDATE habits SET name=?2, icon=?3, color=?4, schedule_type=?5, schedule_data=?6, reminder_time=?7 WHERE id=?1",
        params![
            id, input.name.trim(), input.icon, input.color,
            input.schedule_type, input.schedule_data, input.reminder_time
        ],
    )?;
    conn.query_row("SELECT * FROM habits WHERE id=?1", params![id], row_to_habit)
        .map_err(Into::into)
}

pub fn archive_habit(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("UPDATE habits SET archived_at=?2 WHERE id=?1", params![id, now_iso()])?;
    Ok(())
}

/// Toggle completion for a habit on a given local date. Returns `true` when the
/// habit is completed after the toggle.
pub fn toggle_habit_completion(conn: &Connection, habit_id: &str, date: &str) -> AppResult<bool> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM habit_completions WHERE habit_id=?1 AND completed_date=?2",
            params![habit_id, date],
            |r| r.get(0),
        )
        .optional()?;

    if let Some(id) = existing {
        conn.execute("DELETE FROM habit_completions WHERE id=?1", params![id])?;
        Ok(false)
    } else {
        conn.execute(
            "INSERT INTO habit_completions (id, habit_id, completed_date, completed_at) VALUES (?1,?2,?3,?4)",
            params![uuid::Uuid::new_v4().to_string(), habit_id, date, now_iso()],
        )?;
        Ok(true)
    }
}

pub fn habit_completion_dates(conn: &Connection, habit_id: &str) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT completed_date FROM habit_completions WHERE habit_id=?1 ORDER BY completed_date ASC",
    )?;
    let rows = stmt.query_map(params![habit_id], |r| r.get::<_, String>(0))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Active habits that have a reminder time configured, for the scheduler.
pub fn habits_with_reminder(conn: &Connection) -> AppResult<Vec<Habit>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM habits
         WHERE archived_at IS NULL AND reminder_time IS NOT NULL AND reminder_time != ''",
    )?;
    let rows = stmt.query_map([], row_to_habit)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn habit_completed_on(conn: &Connection, habit_id: &str, date: &str) -> AppResult<bool> {
    let found: Option<String> = conn
        .query_row(
            "SELECT id FROM habit_completions WHERE habit_id=?1 AND completed_date=?2",
            params![habit_id, date],
            |r| r.get(0),
        )
        .optional()?;
    Ok(found.is_some())
}

pub fn habit_reminder_logged(conn: &Connection, habit_id: &str, date: &str) -> AppResult<bool> {
    let found: Option<String> = conn
        .query_row(
            "SELECT habit_id FROM habit_reminder_log WHERE habit_id=?1 AND fire_date=?2",
            params![habit_id, date],
            |r| r.get(0),
        )
        .optional()?;
    Ok(found.is_some())
}

pub fn log_habit_reminder(conn: &Connection, habit_id: &str, date: &str) -> AppResult<()> {
    conn.execute(
        "INSERT OR IGNORE INTO habit_reminder_log (habit_id, fire_date, fired_at) VALUES (?1, ?2, ?3)",
        params![habit_id, date, now_iso()],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Focus sessions
// ---------------------------------------------------------------------------

fn row_to_focus(row: &rusqlite::Row) -> rusqlite::Result<FocusSession> {
    Ok(FocusSession {
        id: row.get("id")?,
        started_at: row.get("started_at")?,
        ended_at: row.get("ended_at")?,
        planned_seconds: row.get("planned_seconds")?,
        actual_seconds: row.get("actual_seconds")?,
        session_type: row.get("type")?,
        completed: row.get::<_, i64>("completed")? != 0,
    })
}

pub fn start_focus_session(conn: &Connection, input: &FocusSessionInput) -> AppResult<FocusSession> {
    let session = FocusSession {
        id: uuid::Uuid::new_v4().to_string(),
        started_at: now_iso(),
        ended_at: None,
        planned_seconds: input.planned_seconds,
        actual_seconds: None,
        session_type: input.session_type.clone(),
        completed: false,
    };
    conn.execute(
        "INSERT INTO focus_sessions (id, started_at, ended_at, planned_seconds, actual_seconds, type, completed)
         VALUES (?1,?2,NULL,?3,NULL,?4,0)",
        params![session.id, session.started_at, session.planned_seconds, session.session_type],
    )?;
    Ok(session)
}

pub fn end_focus_session(conn: &Connection, id: &str, actual_seconds: i64, completed: bool) -> AppResult<Option<FocusSession>> {
    conn.execute(
        "UPDATE focus_sessions SET ended_at=?2, actual_seconds=?3, completed=?4 WHERE id=?1",
        params![id, now_iso(), actual_seconds, completed as i64],
    )?;
    let r = conn
        .query_row("SELECT * FROM focus_sessions WHERE id=?1", params![id], row_to_focus)
        .optional()?;
    Ok(r)
}

pub fn list_focus_sessions(conn: &Connection, limit: i64) -> AppResult<Vec<FocusSession>> {
    let mut stmt = conn.prepare("SELECT * FROM focus_sessions ORDER BY started_at DESC LIMIT ?1")?;
    let rows = stmt.query_map(params![limit], row_to_focus)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn focus_seconds_since(conn: &Connection, since: &str) -> AppResult<i64> {
    let total: i64 = conn.query_row(
        "SELECT IFNULL(SUM(IFNULL(actual_seconds,0)),0) FROM focus_sessions
         WHERE started_at >= ?1 AND type='focus' AND completed=1",
        params![since],
        |r| r.get(0),
    )?;
    Ok(total)
}

// ---------------------------------------------------------------------------
// Sounds & settings
// ---------------------------------------------------------------------------

pub fn list_sounds(conn: &Connection) -> AppResult<Vec<Sound>> {
    let mut stmt = conn.prepare("SELECT * FROM sounds ORDER BY bundled DESC, name ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok(Sound {
            id: row.get("id")?,
            name: row.get("name")?,
            file_path: row.get("file_path")?,
            bundled: row.get::<_, i64>("bundled")? != 0,
            enabled: row.get::<_, i64>("enabled")? != 0,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn insert_sound(conn: &Connection, name: &str, file_path: &str) -> AppResult<Sound> {
    let sound = Sound {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        file_path: Some(file_path.to_string()),
        bundled: false,
        enabled: true,
    };
    conn.execute(
        "INSERT INTO sounds (id, name, file_path, bundled, enabled) VALUES (?1,?2,?3,0,1)",
        params![sound.id, sound.name, sound.file_path],
    )?;
    Ok(sound)
}

pub fn delete_sound(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM sounds WHERE id=?1 AND bundled=0", params![id])?;
    Ok(())
}

pub fn all_settings(conn: &Connection) -> AppResult<HashMap<String, String>> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut map = HashMap::new();
    for r in rows {
        let (k, v) = r?;
        map.insert(k, v);
    }
    Ok(map)
}

pub fn put_setting(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, value],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// History & aggregates
// ---------------------------------------------------------------------------

pub fn history(conn: &Connection, since: Option<&str>) -> AppResult<Vec<HistoryEntry>> {
    let cutoff = since.unwrap_or("0000-01-01T00:00:00Z").to_string();
    let mut out = Vec::new();

    {
        let mut stmt = conn.prepare(
            "SELECT id, title, status, IFNULL(completed_at, updated_at) AS at FROM reminders
             WHERE status IN ('completed','skipped','dismissed') AND IFNULL(completed_at, updated_at) >= ?1
             ORDER BY at DESC LIMIT 500",
        )?;
        let rows = stmt.query_map(params![cutoff], |r| {
            Ok(HistoryEntry {
                id: r.get(0)?,
                kind: "reminder".into(),
                title: r.get(1)?,
                at: r.get(3)?,
                detail: Some(r.get::<_, String>(2)?),
            })
        })?;
        for r in rows {
            out.push(r?);
        }
    }

    {
        let mut stmt = conn.prepare(
            "SELECT id, started_at, type, IFNULL(actual_seconds,0), completed FROM focus_sessions
             WHERE started_at >= ?1 ORDER BY started_at DESC LIMIT 500",
        )?;
        let rows = stmt.query_map(params![cutoff], |r| {
            let secs: i64 = r.get(3)?;
            let completed: i64 = r.get(4)?;
            Ok(HistoryEntry {
                id: r.get(0)?,
                kind: "focus".into(),
                title: format!("{} session", r.get::<_, String>(2)?),
                at: r.get(1)?,
                detail: Some(format!("{} min · {}", secs / 60, if completed != 0 { "completed" } else { "stopped" })),
            })
        })?;
        for r in rows {
            out.push(r?);
        }
    }

    {
        let mut stmt = conn.prepare(
            "SELECT hc.id, h.name, hc.completed_at FROM habit_completions hc
             JOIN habits h ON h.id = hc.habit_id
             WHERE hc.completed_at >= ?1 ORDER BY hc.completed_at DESC LIMIT 500",
        )?;
        let rows = stmt.query_map(params![cutoff], |r| {
            Ok(HistoryEntry {
                id: r.get(0)?,
                kind: "habit".into(),
                title: r.get(1)?,
                at: r.get(2)?,
                detail: Some("completed".into()),
            })
        })?;
        for r in rows {
            out.push(r?);
        }
    }

    out.sort_by(|a, b| b.at.cmp(&a.at));
    Ok(out)
}

/// Daily completion counts for the lightweight activity graph.
pub fn activity_counts(conn: &Connection, days: i64) -> AppResult<Vec<(String, i64)>> {
    let mut out = Vec::new();
    for offset in (0..days).rev() {
        let date = today_local() - Duration::days(offset);
        let key = date.format("%Y-%m-%d").to_string();
        let count: i64 = conn.query_row(
            "SELECT
                (SELECT COUNT(*) FROM habit_completions WHERE completed_date = ?1) +
                (SELECT COUNT(*) FROM focus_sessions WHERE substr(IFNULL(ended_at, started_at),1,10) = ?1 AND completed=1) +
                (SELECT COUNT(*) FROM reminders WHERE substr(IFNULL(completed_at,''),1,10) = ?1)",
            params![key],
            |r| r.get(0),
        )?;
        out.push((key, count));
    }
    Ok(out)
}

/// Compute week stats for every active habit.
pub fn habits_with_stats(conn: &Connection) -> AppResult<Vec<HabitWithStats>> {
    let habits = list_habits(conn)?;
    let today = today_local();
    let monday = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    let week: Vec<NaiveDate> = (0..7).map(|i| monday + Duration::days(i)).collect();
    let week_strings: Vec<String> = week.iter().map(|d| d.format("%Y-%m-%d").to_string()).collect();
    let today_str = today.format("%Y-%m-%d").to_string();

    let mut out = Vec::new();
    for habit in habits {
        let dates = habit_completion_dates(conn, &habit.id)?;
        let set: std::collections::HashSet<&String> = dates.iter().collect();
        let completed_this_week = week_strings.iter().filter(|d| set.contains(d)).count() as i64;
        let week_completed: Vec<String> = week_strings
            .iter()
            .filter(|d| set.contains(d))
            .cloned()
            .collect();

        out.push(HabitWithStats {
            target_this_week: target_for(&habit, &week_strings),
            completed_today: set.contains(&today_str),
            current_streak: streak_from(&dates),
            best_streak: best_streak(&dates),
            completed_this_week,
            week_dates: week_strings.clone(),
            week_completed,
            habit,
        });
    }
    Ok(out)
}

fn target_for(habit: &Habit, week: &[String]) -> i64 {
    match habit.schedule_type.as_str() {
        "times_per_week" => serde_json::from_str::<serde_json::Value>(&habit.schedule_data)
            .ok()
            .and_then(|v| v.get("times").and_then(|t| t.as_i64()))
            .unwrap_or(7),
        "weekdays" => week.iter().filter(|d| is_weekday_str(d)).count() as i64,
        "custom" => {
            let days: Vec<String> = serde_json::from_str(&habit.schedule_data).unwrap_or_default();
            week.iter()
                .filter(|d| {
                    let wd = weekday_short(d);
                    days.iter().any(|x| x.eq_ignore_ascii_case(&wd))
                })
                .count() as i64
        }
        _ => 7,
    }
}

fn is_weekday_str(date: &str) -> bool {
    let wd = weekday_short(date);
    !matches!(wd.as_str(), "SA" | "SU")
}

fn weekday_short(date: &str) -> String {
    match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        Ok(d) => match d.weekday() {
            chrono::Weekday::Mon => "MO",
            chrono::Weekday::Tue => "TU",
            chrono::Weekday::Wed => "WE",
            chrono::Weekday::Thu => "TH",
            chrono::Weekday::Fri => "FR",
            chrono::Weekday::Sat => "SA",
            chrono::Weekday::Sun => "SU",
        }
        .to_string(),
        Err(_) => String::new(),
    }
}

/// Consecutive days ending today (or yesterday, so an unfinished today doesn't
/// break a streak).
fn streak_from(dates: &[String]) -> i64 {
    if dates.is_empty() {
        return 0;
    }
    let set: std::collections::HashSet<&str> = dates.iter().map(|s| s.as_str()).collect();
    let today = today_local();
    let mut cursor = if set.contains(today.format("%Y-%m-%d").to_string().as_str()) {
        today
    } else {
        today - Duration::days(1)
    };
    let mut streak = 0;
    while set.contains(cursor.format("%Y-%m-%d").to_string().as_str()) {
        streak += 1;
        cursor -= Duration::days(1);
    }
    streak
}

fn best_streak(dates: &[String]) -> i64 {
    let mut sorted: Vec<NaiveDate> = dates
        .iter()
        .filter_map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .collect();
    sorted.sort();
    sorted.dedup();
    let mut best = 0;
    let mut run = 0;
    let mut prev: Option<NaiveDate> = None;
    for d in sorted {
        run = match prev {
            Some(p) if d == p + Duration::days(1) => run + 1,
            _ => 1,
        };
        best = best.max(run);
        prev = Some(d);
    }
    best
}
