use chrono::{DateTime, Datelike, Duration, Months, TimeZone, Utc, Weekday};

/// A parsed subset of RFC 5545 RRULE, limited to the patterns the UI produces:
/// `FREQ=HOURLY|DAILY|WEEKLY|MONTHLY` with optional `INTERVAL` and `BYDAY`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurrenceRule {
    pub freq: Freq,
    pub interval: u32,
    pub by_day: Vec<Weekday>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freq {
    Hourly,
    Daily,
    Weekly,
    Monthly,
}

impl RecurrenceRule {
    /// Parse a rule string. Unknown tokens are ignored for forward compatibility.
    pub fn parse(rule: &str) -> Option<Self> {
        let mut freq = None;
        let mut interval = 1u32;
        let mut by_day = Vec::new();

        for part in rule.split(';') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let (key, value) = match part.split_once('=') {
                Some(kv) => kv,
                None => continue,
            };
            match key.trim().to_ascii_uppercase().as_str() {
                "FREQ" => {
                    freq = match value.trim().to_ascii_uppercase().as_str() {
                        "HOURLY" => Some(Freq::Hourly),
                        "DAILY" => Some(Freq::Daily),
                        "WEEKLY" => Some(Freq::Weekly),
                        "MONTHLY" => Some(Freq::Monthly),
                        _ => None,
                    }
                }
                "INTERVAL" => {
                    if let Ok(v) = value.trim().parse::<u32>() {
                        if v > 0 {
                            interval = v;
                        }
                    }
                }
                "BYDAY" => {
                    for d in value.split(',') {
                        if let Some(wd) = parse_weekday(d.trim()) {
                            by_day.push(wd);
                        }
                    }
                }
                _ => {}
            }
        }

        freq.map(|freq| RecurrenceRule { freq, interval, by_day })
    }

    /// The next occurrence strictly after `after`, anchored to the original due
    /// time so intervals stay aligned.
    pub fn next_after(&self, anchor: DateTime<Utc>, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let mut cursor = anchor;

        // Guard against pathological rules / clock skew.
        for _ in 0..200_000 {
            cursor = self.advance(cursor)?;
            if cursor > after {
                return Some(cursor);
            }
        }
        None
    }

    fn advance(&self, from: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self.freq {
            Freq::Hourly => from.checked_add_signed(Duration::hours(self.interval as i64)),
            Freq::Daily => from.checked_add_signed(Duration::days(self.interval as i64)),
            Freq::Monthly => from.checked_add_months(Months::new(self.interval)),
            Freq::Weekly => {
                if self.by_day.is_empty() {
                    return from.checked_add_signed(Duration::days(7 * self.interval as i64));
                }
                let mut candidate = from.checked_add_signed(Duration::days(1))?;
                let anchor_week_start = week_start(from);
                for _ in 0..(7 * self.interval * 4000).max(7) {
                    let weeks = (week_start(candidate) - anchor_week_start).num_weeks();
                    if weeks >= 0
                        && (weeks as u32) % self.interval == 0
                        && self.by_day.contains(&candidate.weekday())
                    {
                        return Some(candidate);
                    }
                    candidate = candidate.checked_add_signed(Duration::days(1))?;
                }
                None
            }
        }
    }
}

fn week_start(dt: DateTime<Utc>) -> DateTime<Utc> {
    let days = dt.weekday().num_days_from_monday() as i64;
    let date = dt.date_naive() - Duration::days(days);
    Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
}

fn parse_weekday(s: &str) -> Option<Weekday> {
    match s.to_ascii_uppercase().as_str() {
        "MO" | "MON" => Some(Weekday::Mon),
        "TU" | "TUE" => Some(Weekday::Tue),
        "WE" | "WED" => Some(Weekday::Wed),
        "TH" | "THU" => Some(Weekday::Thu),
        "FR" | "FRI" => Some(Weekday::Fri),
        "SA" | "SAT" => Some(Weekday::Sat),
        "SU" | "SUN" => Some(Weekday::Sun),
        _ => None,
    }
}

/// Advance an occurrence past `now`, returning the next due timestamp and
/// whether the rule still applies.
pub fn next_due(rule: &str, anchor: DateTime<Utc>, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    RecurrenceRule::parse(rule)?.next_after(anchor, now)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn parses_basic_rules() {
        let r = RecurrenceRule::parse("FREQ=DAILY").unwrap();
        assert_eq!(r.freq, Freq::Daily);
        assert_eq!(r.interval, 1);

        let r = RecurrenceRule::parse("FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,FR").unwrap();
        assert_eq!(r.freq, Freq::Weekly);
        assert_eq!(r.interval, 2);
        assert_eq!(r.by_day, vec![Weekday::Mon, Weekday::Fri]);

        assert!(RecurrenceRule::parse("NONSENSE=1").is_none());
    }

    #[test]
    fn daily_next_occurrence() {
        let anchor = dt("2026-10-01T09:00:00Z");
        let next = next_due("FREQ=DAILY", anchor, anchor).unwrap();
        assert_eq!(next, dt("2026-10-02T09:00:00Z"));
    }

    #[test]
    fn hourly_skips_missed_occurrences() {
        let anchor = dt("2026-10-01T09:00:00Z");
        let now = dt("2026-10-01T14:30:00Z");
        let next = next_due("FREQ=HOURLY;INTERVAL=2", anchor, now).unwrap();
        assert_eq!(next, dt("2026-10-01T15:00:00Z"));
    }

    #[test]
    fn weekday_rule_skips_weekend() {
        // Friday 2026-10-02 17:00 -> next weekday is Monday 2026-10-05.
        let anchor = dt("2026-10-02T17:00:00Z");
        let next = next_due("FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR", anchor, anchor).unwrap();
        assert_eq!(next, dt("2026-10-05T17:00:00Z"));
    }

    #[test]
    fn monthly_advances_by_month() {
        let anchor = dt("2026-10-15T08:00:00Z");
        let next = next_due("FREQ=MONTHLY", anchor, anchor).unwrap();
        assert_eq!(next, dt("2026-11-15T08:00:00Z"));
    }
}
