use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use chrono::{DateTime, Datelike, Local, NaiveTime, Timelike, Utc};
use rusqlite::Connection;
use tauri::{AppHandle, Emitter, Manager};

use crate::db;
use crate::error::AppResult;
use crate::focus::Focus;
use crate::microbreak::MicroBreak;
use crate::models::Reminder;
use crate::notify::{self, Kind};
use crate::pending::{self, PendingAction};
use crate::prefs::RuntimePrefs;
use crate::recurrence::next_due;
use crate::sound::{self, SoundSlot};

/// Upper bound on how long the scheduler sleeps before re-checking the wall
/// clock. This is a low-frequency safety net for suspend/resume and clock
/// changes — not a polling loop — and is only reached when no reminder is due
/// sooner. It also bounds how late a minute-granularity habit reminder can be.
const MAX_SLEEP: Duration = Duration::from_secs(60);

/// Wakes the scheduler immediately when reminder data changes.
#[derive(Clone)]
pub struct SchedulerHandle {
    inner: Arc<Inner>,
}

struct Inner {
    lock: Mutex<()>,
    cv: Condvar,
    stop: AtomicBool,
}

impl SchedulerHandle {
    pub fn wake(&self) {
        self.inner.cv.notify_all();
    }
}

/// Spawn the scheduler thread. The thread owns its own SQLite connection, which
/// is safe with WAL mode and keeps the scheduler independent from UI commands.
pub fn spawn(
    app: AppHandle,
    db_path: PathBuf,
    prefs: Arc<RuntimePrefs>,
    micro: Arc<MicroBreak>,
    focus: Arc<Focus>,
) -> SchedulerHandle {
    let handle = SchedulerHandle {
        inner: Arc::new(Inner {
            lock: Mutex::new(()),
            cv: Condvar::new(),
            stop: AtomicBool::new(false),
        }),
    };

    let worker = handle.clone();
    std::thread::Builder::new()
        .name("reminder-scheduler".into())
        .spawn(move || {
            let conn = match db::open(&db_path) {
                Ok(conn) => conn,
                Err(err) => {
                    eprintln!("scheduler: failed to open database: {err}");
                    return;
                }
            };
            run_loop(&app, &conn, &worker, &prefs, &micro, &focus);
        })
        .expect("failed to spawn scheduler thread");

    handle
}

fn run_loop(
    app: &AppHandle,
    conn: &Connection,
    handle: &SchedulerHandle,
    prefs: &Arc<RuntimePrefs>,
    micro: &Arc<MicroBreak>,
    focus: &Arc<Focus>,
) {
    loop {
        if handle.inner.stop.load(Ordering::SeqCst) {
            return;
        }

        let now = Utc::now();
        let local_now = Local::now();
        if let Err(err) = process_due(app, conn, prefs, now) {
            eprintln!("scheduler: processing reminders failed: {err}");
        }
        if let Err(err) = process_habit_reminders(app, conn, prefs, local_now) {
            eprintln!("scheduler: processing habits failed: {err}");
        }
        process_micro_break(app, conn, micro, prefs, local_now);
        process_focus(app, conn, prefs, focus);

        let wait = next_wait(conn, now, micro.next_at(), focus.deadline());
        let guard = handle.inner.lock.lock().unwrap();
        let _ = handle.inner.cv.wait_timeout(guard, wait);
    }
}

// ---------------------------------------------------------------------------
// Reminders
// ---------------------------------------------------------------------------

fn process_due(
    app: &AppHandle,
    conn: &Connection,
    prefs: &Arc<RuntimePrefs>,
    now: DateTime<Utc>,
) -> AppResult<()> {
    let now_str = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let due = db::due_reminders(conn, &now_str)?;
    if due.is_empty() {
        return Ok(());
    }

    for reminder in due {
        fire_reminder(app, conn, prefs, &reminder);

        let anchor = DateTime::parse_from_rfc3339(&reminder.due_at)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or(now);

        match reminder
            .recurrence_rule
            .as_deref()
            .and_then(|rule| next_due(rule, anchor, now))
        {
            Some(next) => {
                let next_str = next.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                conn.execute(
                    "UPDATE reminders SET due_at=?2, status='scheduled', updated_at=?3 WHERE id=?1",
                    rusqlite::params![reminder.id, next_str, now_str],
                )?;
            }
            None => {
                // One-shot reminder: it is now overdue and waits for the user.
                db::set_status(conn, &reminder.id, "overdue", None)?;
            }
        }
    }

    let _ = app.emit("reminders-changed", ());
    Ok(())
}

fn fire_reminder(app: &AppHandle, conn: &Connection, prefs: &Arc<RuntimePrefs>, reminder: &Reminder) {
    // The in-app toast always fires; only the OS notification is gated.
    let _ = app.emit("reminder-fired", reminder.clone());

    // When the window is closed the webview is gone, so the frontend cannot
    // play the sound — do it from here instead.
    sound::play_if_hidden(app, conn, prefs, SoundSlot::Reminder, Local::now());

    if !reminder.notification_enabled {
        return;
    }
    if prefs.snapshot().notifications_suppressed() {
        return;
    }

    let body = reminder
        .notes
        .as_deref()
        .filter(|n| !n.trim().is_empty())
        .unwrap_or("It's time.");

    // Pretty XFCE bubble: "⏰ Title" summary, notes body, app icon, urgency.
    let title = format!("⏰ {}", reminder.title);
    notify::send(&title, body, Kind::Reminder, notify::theme_sound(reminder.sound_id.as_deref()));
}

// ---------------------------------------------------------------------------
// Habits
// ---------------------------------------------------------------------------

fn process_habit_reminders(
    app: &AppHandle,
    conn: &Connection,
    prefs: &Arc<RuntimePrefs>,
    now: DateTime<Local>,
) -> AppResult<()> {
    let habits = db::habits_with_reminder(conn)?;
    if habits.is_empty() {
        return Ok(());
    }

    let date = now.format("%Y-%m-%d").to_string();
    let now_minutes = now.hour() * 60 + now.minute();
    let weekday = weekday_code(now);

    for habit in habits {
        let Some(raw) = habit.reminder_time.as_deref() else {
            continue;
        };
        let Ok(time) = NaiveTime::parse_from_str(raw.trim(), "%H:%M") else {
            eprintln!("scheduler: habit {} has an invalid reminder time", habit.id);
            continue;
        };
        if now_minutes < time.hour() * 60 + time.minute() {
            continue;
        }
        if !is_scheduled_today(&habit, weekday) {
            continue;
        }
        if db::habit_reminder_logged(conn, &habit.id, &date)? {
            continue;
        }

        // Log before notifying so a failure can never loop.
        db::log_habit_reminder(conn, &habit.id, &date)?;

        // Already checked off today? Nothing to remind about.
        if db::habit_completed_on(conn, &habit.id, &date)? {
            continue;
        }

        let _ = app.emit(
            "habit-reminder-fired",
            serde_json::json!({
                "id": habit.id,
                "name": habit.name,
                "icon": habit.icon,
            }),
        );

        sound::play_if_hidden(app, conn, prefs, SoundSlot::Habit, now);

        if prefs.snapshot().notifications_suppressed() {
            continue;
        }

        let title = format!("{} {}", habit.icon.as_deref().unwrap_or("🌱"), habit.name);
        notify::send(&title, "Time for your habit — small steps count.", Kind::Nudge, Some("message-new-instant"));
    }

    Ok(())
}

fn weekday_code(date: DateTime<Local>) -> &'static str {
    match date.weekday() {
        chrono::Weekday::Mon => "MO",
        chrono::Weekday::Tue => "TU",
        chrono::Weekday::Wed => "WE",
        chrono::Weekday::Thu => "TH",
        chrono::Weekday::Fri => "FR",
        chrono::Weekday::Sat => "SA",
        chrono::Weekday::Sun => "SU",
    }
}

fn is_scheduled_today(habit: &crate::models::Habit, weekday: &str) -> bool {
    match habit.schedule_type.as_str() {
        "weekdays" => !matches!(weekday, "SA" | "SU"),
        "custom" => {
            let days: Vec<String> = serde_json::from_str(&habit.schedule_data).unwrap_or_default();
            days.iter().any(|d| d.eq_ignore_ascii_case(weekday))
        }
        // `daily` and `times_per_week` both allow any day.
        _ => true,
    }
}

// ---------------------------------------------------------------------------
// Micro breaks
// ---------------------------------------------------------------------------

/// Micro breaks are deliberately *not* a second Pomodoro: they only need to
/// nudge, so this is one timestamp and a notification rather than a state
/// machine. The overlay and its countdown live in the UI.
fn process_micro_break(
    app: &AppHandle,
    conn: &Connection,
    micro: &Arc<MicroBreak>,
    prefs: &Arc<RuntimePrefs>,
    now: DateTime<Local>,
) {
    let snap = prefs.snapshot();
    if !snap.micro_breaks_enabled {
        micro.clear();
        return;
    }

    let work = chrono::Duration::minutes(i64::from(snap.micro_work_minutes));
    if !micro.take_due(work, now) {
        return;
    }

    // Quiet hours silence the nudge entirely — a break prompt at 2am is noise.
    if snap.in_quiet_hours(now) {
        return;
    }

    let break_seconds = i64::from(snap.micro_break_minutes) * 60;

    // A due break should surface: if a window can show the overlay, hand it
    // over immediately; otherwise rebuild the window around the overlay (the
    // app may have been started minimized, before any webview existed).
    // Purely minimized windows are restored as normal.
    match app.get_webview_window("main") {
        Some(window) => {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
            pending::set_ready(true);
            let _ = app.emit(
                "micro-break-due",
                serde_json::json!({ "breakSeconds": break_seconds }),
            );
        }
        None => {
            if !crate::pending_window(app, PendingAction::MicroBreakDue { break_seconds }) {
                let _ = app.emit(
                    "micro-break-due",
                    serde_json::json!({ "breakSeconds": break_seconds }),
                );
            }
        }
    }

    // Only reached outside quiet hours (checked above), so this is safe.
    sound::play_if_hidden(app, conn, prefs, SoundSlot::MicroBreak, now);

    if !snap.notifications_enabled {
        return;
    }

    notify::send(
        "☕ Time for a break",
        "Look away from the screen for a few minutes — your eyes will thank you.",
        Kind::Nudge,
        Some("message-new-instant"),
    );
}

// ---------------------------------------------------------------------------
// Focus
// ---------------------------------------------------------------------------

/// Fire a due focus phase boundary. The focus timer lives in Rust (see
/// `focus.rs`), so phase transitions happen here even with no webview alive.
fn process_focus(
    app: &AppHandle,
    conn: &Connection,
    prefs: &Arc<RuntimePrefs>,
    focus: &Arc<Focus>,
) {
    if !focus.due(Utc::now()) {
        return;
    }
    crate::focus::finish_phase(conn, prefs, app, focus, true);
}

// ---------------------------------------------------------------------------
// Scheduling
// ---------------------------------------------------------------------------

fn next_wait(
    conn: &Connection,
    now: DateTime<Utc>,
    micro_at: Option<DateTime<Local>>,
    focus_at: Option<DateTime<Utc>>,
) -> Duration {
    let mut wait = MAX_SLEEP;

    match db::next_due_at(conn) {
        Ok(Some(ts)) => match DateTime::parse_from_rfc3339(&ts) {
            Ok(target) => {
                let delta = target.with_timezone(&Utc) - now;
                wait = Duration::from_millis(delta.num_milliseconds().max(0) as u64);
            }
            Err(_) => {}
        },
        Ok(None) => {}
        Err(err) => eprintln!("scheduler: lookup failed: {err}"),
    }

    if let Some(at) = micro_at {
        let delta = at - Local::now();
        wait = wait.min(Duration::from_millis(delta.num_milliseconds().max(0) as u64));
    }

    if let Some(at) = focus_at {
        let delta = at - Utc::now();
        wait = wait.min(Duration::from_millis(delta.num_milliseconds().max(0) as u64));
    }

    wait.clamp(Duration::from_millis(250), MAX_SLEEP)
}
