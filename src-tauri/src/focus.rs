//! Authoritative focus (Pomodoro) timer.
//!
//! Deadline model: Rust owns phase, running state, and the wall-clock
//! deadline. The scheduler fires phase boundaries; the UI only interpolates
//! the countdown for display and re-syncs on `focus-changed`. Destroying the
//! webview therefore never loses the timer.
//!
//! The phase machine mirrors the old frontend loop in `src/store.ts`: a
//! finished focus phase counts toward the long-break cycle (even when
//! skipped), a non-auto-started phase parks un-running with no DB row, and
//! only a natural focus end chimes.

use std::sync::{Arc, Mutex};

use chrono::{DateTime, Local, Utc};
use rusqlite::Connection;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::db;
use crate::models::FocusSessionInput;
use crate::prefs::{Prefs, RuntimePrefs};
use crate::sound::SoundSlot;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FocusPhase {
    Idle,
    Focus,
    ShortBreak,
    LongBreak,
}

impl FocusPhase {
    fn as_str(self) -> &'static str {
        match self {
            FocusPhase::Idle => "idle",
            FocusPhase::Focus => "focus",
            FocusPhase::ShortBreak => "short_break",
            FocusPhase::LongBreak => "long_break",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusStatus {
    pub phase: FocusPhase,
    pub running: bool,
    pub remaining: i64,
    pub planned: i64,
    pub session_id: Option<String>,
    pub completed_in_cycle: i32,
}

struct Inner {
    phase: FocusPhase,
    running: bool,
    ends_at: Option<DateTime<Utc>>,
    remaining: i64,
    planned: i64,
    session_id: Option<String>,
    completed_in_cycle: i32,
}

/// A finished phase captured for closing out its DB row.
pub struct FinishedPhase {
    phase: FocusPhase,
    planned: i64,
    remaining: i64,
    session_id: Option<String>,
    completed_in_cycle: i32,
}

pub struct Focus {
    inner: Mutex<Inner>,
}

impl Focus {
    pub fn new() -> Self {
        Focus {
            inner: Mutex::new(Inner {
                phase: FocusPhase::Idle,
                running: false,
                ends_at: None,
                remaining: 0,
                planned: 0,
                session_id: None,
                completed_in_cycle: 0,
            }),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn status_at(&self, now: DateTime<Utc>) -> FocusStatus {
        let inner = self.lock();
        FocusStatus {
            phase: inner.phase,
            running: inner.running,
            remaining: remaining_locked(&inner, now),
            planned: inner.planned,
            session_id: inner.session_id.clone(),
            completed_in_cycle: inner.completed_in_cycle,
        }
    }

    pub fn status(&self) -> FocusStatus {
        self.status_at(Utc::now())
    }

    /// Wall-clock deadline the scheduler waits on, if the timer is running.
    pub fn deadline(&self) -> Option<DateTime<Utc>> {
        let inner = self.lock();
        if inner.running {
            inner.ends_at
        } else {
            None
        }
    }

    pub fn is_running(&self) -> bool {
        self.lock().running
    }

    pub fn due(&self, now: DateTime<Utc>) -> bool {
        let inner = self.lock();
        inner.running && inner.ends_at.is_some_and(|at| now >= at)
    }

    fn completed_in_cycle(&self) -> i32 {
        self.lock().completed_in_cycle
    }

    /// Snapshot the current phase and stop the clock. The caller closes the
    /// DB row and applies the next phase. Returns `None` when idle.
    fn take_finished(&self, now: DateTime<Utc>) -> Option<FinishedPhase> {
        let mut inner = self.lock();
        if inner.phase == FocusPhase::Idle {
            return None;
        }
        let finished = FinishedPhase {
            phase: inner.phase,
            planned: inner.planned,
            remaining: remaining_locked(&inner, now),
            session_id: inner.session_id.take(),
            completed_in_cycle: inner.completed_in_cycle,
        };
        inner.running = false;
        inner.ends_at = None;
        inner.remaining = finished.remaining;
        Some(finished)
    }

    fn apply_next(
        &self,
        now: DateTime<Utc>,
        phase: FocusPhase,
        planned: i64,
        running: bool,
        session_id: Option<String>,
        completed_in_cycle: i32,
    ) {
        let mut inner = self.lock();
        inner.phase = phase;
        inner.planned = planned;
        inner.running = running;
        inner.session_id = session_id;
        inner.completed_in_cycle = completed_in_cycle;
        if running {
            inner.ends_at = Some(now + chrono::Duration::seconds(planned.max(1)));
            inner.remaining = planned;
        } else {
            inner.ends_at = None;
            inner.remaining = planned;
        }
    }

    fn pause_at(&self, now: DateTime<Utc>) -> bool {
        let mut inner = self.lock();
        if !inner.running {
            return false;
        }
        inner.remaining = remaining_locked(&inner, now);
        inner.running = false;
        inner.ends_at = None;
        true
    }

    fn resume_at(&self, now: DateTime<Utc>) -> bool {
        let mut inner = self.lock();
        if inner.phase == FocusPhase::Idle || inner.running {
            return false;
        }
        inner.running = true;
        inner.ends_at = Some(now + chrono::Duration::seconds(inner.remaining.max(1)));
        true
    }

    fn reset(&self) {
        let mut inner = self.lock();
        inner.phase = FocusPhase::Idle;
        inner.running = false;
        inner.ends_at = None;
        inner.remaining = 0;
        inner.planned = 0;
        inner.session_id = None;
        inner.completed_in_cycle = 0;
    }
}

impl Default for Focus {
    fn default() -> Self {
        Self::new()
    }
}

fn remaining_locked(inner: &Inner, now: DateTime<Utc>) -> i64 {
    if inner.running {
        match inner.ends_at {
            Some(at) => (at - now).num_seconds().max(0),
            None => inner.remaining.max(0),
        }
    } else {
        inner.remaining.max(0)
    }
}

// ---------------------------------------------------------------------------
// Phase machine (pure: mirrors the old `finishPhase` in `src/store.ts`)
// ---------------------------------------------------------------------------

/// Which phase follows `prev` once `completed` focus phases are done.
pub fn next_phase(prev: FocusPhase, completed: i32, sessions_before_long: i64) -> FocusPhase {
    if prev == FocusPhase::Focus {
        if completed % sessions_before_long.max(1) as i32 == 0 {
            FocusPhase::LongBreak
        } else {
            FocusPhase::ShortBreak
        }
    } else {
        FocusPhase::Focus
    }
}

pub fn phase_planned(phase: FocusPhase, prefs: &Prefs) -> i64 {
    let minutes = match phase {
        FocusPhase::Idle => return 0,
        FocusPhase::Focus => i64::from(prefs.focus_minutes.max(1)),
        FocusPhase::ShortBreak => i64::from(prefs.short_break_minutes.max(1)),
        FocusPhase::LongBreak => i64::from(prefs.long_break_minutes.max(1)),
    };
    minutes * 60
}

pub fn auto_start(phase: FocusPhase, prefs: &Prefs) -> bool {
    if phase == FocusPhase::Focus {
        prefs.auto_start_focus
    } else {
        prefs.auto_start_breaks
    }
}

// ---------------------------------------------------------------------------
// Operations shared by commands, tray, and scheduler
// ---------------------------------------------------------------------------

pub fn emit_status(app: &AppHandle, focus: &Focus) {
    let _ = app.emit("focus-changed", focus.status());
}

/// Close the open session row, if any, without ending the logical phase.
fn close_open_row(conn: &Connection, focus: &Focus, completed: bool) {
    let now = Utc::now();
    if let Some(finished) = focus.take_finished(now) {
        if let Some(id) = &finished.session_id {
            let actual = (finished.planned - finished.remaining.max(0)).max(0);
            if let Err(err) = db::end_focus_session(conn, id, actual, completed) {
                eprintln!("focus: failed to close session: {err}");
            }
        }
    }
}

fn open_row(conn: &Connection, phase: FocusPhase, planned: i64) -> Option<String> {
    let input = FocusSessionInput {
        planned_seconds: planned,
        session_type: phase.as_str().to_string(),
    };
    match db::start_focus_session(conn, &input) {
        Ok(session) => Some(session.id),
        Err(err) => {
            eprintln!("focus: failed to open session: {err}");
            None
        }
    }
}

/// Start a fresh focus phase. Any running phase is closed as incomplete first
/// (the old UI loop orphaned its row here).
pub fn start(
    app: &AppHandle,
    conn: &Connection,
    prefs: &Arc<RuntimePrefs>,
    focus: &Focus,
) -> FocusStatus {
    close_open_row(conn, focus, false);
    let snap = prefs.snapshot();
    let planned = phase_planned(FocusPhase::Focus, &snap);
    let cycle = focus.completed_in_cycle();
    let session_id = open_row(conn, FocusPhase::Focus, planned);
    focus.apply_next(Utc::now(), FocusPhase::Focus, planned, true, session_id, cycle);
    emit_status(app, focus);
    focus.status()
}

pub fn pause(app: &AppHandle, focus: &Focus) -> FocusStatus {
    if focus.pause_at(Utc::now()) {
        emit_status(app, focus);
    }
    focus.status()
}

pub fn resume(
    app: &AppHandle,
    conn: &Connection,
    prefs: &Arc<RuntimePrefs>,
    focus: &Focus,
) -> FocusStatus {
    if focus.status().phase == FocusPhase::Idle {
        return start(app, conn, prefs, focus);
    }
    if focus.resume_at(Utc::now()) {
        emit_status(app, focus);
    }
    focus.status()
}

/// Mirrors the old `focusToggle` semantics exactly.
pub fn toggle(
    app: &AppHandle,
    conn: &Connection,
    prefs: &Arc<RuntimePrefs>,
    focus: &Focus,
    force: Option<bool>,
) -> FocusStatus {
    let running = focus.is_running();
    match force {
        Some(true) if !running => resume(app, conn, prefs, focus),
        Some(false) if running => pause(app, focus),
        Some(_) => focus.status(),
        None => {
            if running {
                pause(app, focus)
            } else {
                resume(app, conn, prefs, focus)
            }
        }
    }
}

pub fn skip(
    app: &AppHandle,
    conn: &Connection,
    prefs: &Arc<RuntimePrefs>,
    focus: &Focus,
) -> FocusStatus {
    if focus.status().phase == FocusPhase::Idle {
        return focus.status();
    }
    finish_phase(conn, prefs, app, focus, false);
    focus.status()
}

pub fn reset(app: &AppHandle, conn: &Connection, focus: &Focus) -> FocusStatus {
    if focus.status().phase == FocusPhase::Idle {
        return focus.status();
    }
    close_open_row(conn, focus, false);
    focus.reset();
    emit_status(app, focus);
    focus.status()
}

/// Advance one phase boundary: close the current row, open the next when
/// auto-start applies, emit, and chime on a natural focus end.
pub fn finish_phase(
    conn: &Connection,
    prefs: &Arc<RuntimePrefs>,
    app: &AppHandle,
    focus: &Focus,
    natural: bool,
) {
    let now = Utc::now();
    let Some(finished) = focus.take_finished(now) else {
        return;
    };
    if let Some(id) = &finished.session_id {
        let actual = (finished.planned - finished.remaining.max(0)).max(0);
        if let Err(err) = db::end_focus_session(conn, id, actual, natural) {
            eprintln!("focus: failed to close session: {err}");
        }
    }

    let snap = prefs.snapshot();
    let completed = if finished.phase == FocusPhase::Focus {
        finished.completed_in_cycle + 1
    } else {
        finished.completed_in_cycle
    };
    let sessions = i64::from(snap.sessions_before_long_break.max(1));
    let next = next_phase(finished.phase, completed, sessions);
    let planned = phase_planned(next, &snap);
    let auto = auto_start(next, &snap);
    let stored = completed % sessions as i32;
    let session_id = if auto {
        open_row(conn, next, planned)
    } else {
        None
    };
    focus.apply_next(Utc::now(), next, planned, auto, session_id, stored);
    emit_status(app, focus);

    // Only a natural focus end chimes — skipping to a break stays silent.
    if natural
        && finished.phase == FocusPhase::Focus
        && snap.sound_enabled
        && !snap.pomodoro_sound.trim().is_empty()
        && snap.pomodoro_sound != "none"
        && !snap.in_quiet_hours(Local::now())
    {
        if crate::pending::is_ready() {
            // The UI is mounted: let it play through WebAudio like before.
            let _ = app.emit("focus-chime", snap.pomodoro_sound.clone());
        } else {
            crate::sound::play_if_hidden(app, conn, prefs, SoundSlot::Pomodoro, Local::now());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(secs: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(secs, 0).single().unwrap()
    }

    fn running_focus() -> Focus {
        let focus = Focus::new();
        focus.apply_next(at(1000), FocusPhase::Focus, 1500, true, Some("s1".into()), 0);
        focus
    }

    #[test]
    fn start_counts_down_to_deadline() {
        let focus = running_focus();
        assert_eq!(focus.status_at(at(1000)).remaining, 1500);
        assert_eq!(focus.status_at(at(1001)).remaining, 1499);
        assert!(!focus.due(at(2499)));
        assert!(focus.due(at(2500)));
    }

    #[test]
    fn pause_stores_remaining_and_resume_reanchors() {
        let focus = running_focus();
        assert!(focus.pause_at(at(1100)));
        assert_eq!(focus.status_at(at(5000)).remaining, 1400);
        assert!(!focus.pause_at(at(5000)));
        assert!(focus.resume_at(at(5000)));
        assert!(!focus.resume_at(at(5000)));
        assert_eq!(focus.status_at(at(5001)).remaining, 1399);
        assert!(focus.due(at(6400)));
    }

    #[test]
    fn resume_from_idle_is_rejected() {
        let focus = Focus::new();
        assert!(!focus.resume_at(at(1000)));
        assert_eq!(focus.status().phase, FocusPhase::Idle);
    }

    #[test]
    fn cycle_picks_short_then_long_break() {
        assert_eq!(next_phase(FocusPhase::Focus, 1, 4), FocusPhase::ShortBreak);
        assert_eq!(next_phase(FocusPhase::Focus, 3, 4), FocusPhase::ShortBreak);
        assert_eq!(next_phase(FocusPhase::Focus, 4, 4), FocusPhase::LongBreak);
        assert_eq!(next_phase(FocusPhase::Focus, 2, 1), FocusPhase::LongBreak);
        assert_eq!(next_phase(FocusPhase::ShortBreak, 1, 4), FocusPhase::Focus);
        assert_eq!(next_phase(FocusPhase::LongBreak, 4, 4), FocusPhase::Focus);
    }

    #[test]
    fn take_finished_stops_clock_and_keeps_snapshot() {
        let focus = running_focus();
        let finished = focus.take_finished(at(1600)).unwrap();
        assert_eq!(finished.phase, FocusPhase::Focus);
        assert_eq!(finished.remaining, 900);
        assert_eq!(finished.session_id.as_deref(), Some("s1"));
        assert!(!focus.is_running());
        // Remaining stays frozen once the clock stops.
        assert_eq!(focus.status_at(at(9000)).remaining, 900);
    }

    #[test]
    fn idle_has_no_deadline() {
        let focus = Focus::new();
        assert_eq!(focus.deadline(), None);
        assert!(!focus.due(at(99999)));
    }
}
