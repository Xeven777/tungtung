use std::sync::Mutex;

use chrono::{DateTime, Duration, Local};

/// A lightweight break nudge that runs alongside Pomodoro rather than competing
/// with it. The work interval restarts whenever the user finishes a break,
/// snoozes, or changes the setting, and it never counts down during quiet hours.
///
/// State is deliberately in-memory: the interval is anchored to this session,
/// which is honest about the fact that the app cannot observe whether the user
/// was actually at the keyboard.
pub struct MicroBreak {
    next_at: Mutex<Option<DateTime<Local>>>,
}

impl MicroBreak {
    pub fn new() -> Self {
        MicroBreak { next_at: Mutex::new(None) }
    }

    fn slot(&self) -> std::sync::MutexGuard<'_, Option<DateTime<Local>>> {
        self.next_at.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Stop the timer entirely (used when the feature is switched off).
    pub fn clear(&self) {
        *self.slot() = None;
    }

    /// Restart the work interval from `now`.
    pub fn reset(&self, work: Duration, now: DateTime<Local>) {
        *self.slot() = Some(now + work);
    }

    /// Ask for a break on the scheduler's next pass.
    pub fn fire_now(&self, now: DateTime<Local>) {
        *self.slot() = Some(now);
    }

    pub fn next_at(&self) -> Option<DateTime<Local>> {
        *self.slot()
    }

    /// Seconds until the next break, or `None` when the timer is not running.
    pub fn remaining(&self, now: DateTime<Local>) -> Option<i64> {
        self.next_at().map(|at| (at - now).num_seconds().max(0))
    }

    /// True exactly once per due time. Starts the interval on the first call so
    /// enabling the feature counts from now rather than firing immediately.
    pub fn take_due(&self, work: Duration, now: DateTime<Local>) -> bool {
        let mut slot = self.slot();
        let due = match *slot {
            None => {
                *slot = Some(now + work);
                return false;
            }
            Some(at) => now >= at,
        };
        if due {
            *slot = Some(now + work);
        }
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(minute: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 10, 2, 9, minute, 0).single().unwrap()
    }

    #[test]
    fn first_poll_starts_the_interval_without_firing() {
        let micro = MicroBreak::new();
        assert!(!micro.take_due(Duration::minutes(50), at(0)));
        assert_eq!(micro.remaining(at(0)), Some(3000));
    }

    #[test]
    fn fires_once_when_due_and_restarts() {
        let micro = MicroBreak::new();
        micro.reset(Duration::minutes(50), at(0));
        assert!(!micro.take_due(Duration::minutes(50), at(49)));
        assert!(micro.take_due(Duration::minutes(50), at(50)));
        // Restarted, so an immediate re-check must not fire again.
        assert!(!micro.take_due(Duration::minutes(50), at(50)));
    }

    #[test]
    fn cleared_timer_reports_nothing() {
        let micro = MicroBreak::new();
        micro.reset(Duration::minutes(50), at(0));
        micro.clear();
        assert_eq!(micro.remaining(at(10)), None);
    }
}
