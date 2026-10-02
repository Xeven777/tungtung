/**
 * A deliberately small, deterministic parser for quick-add input. It recognises
 * the common patterns from the spec and falls back to defaults rather than
 * attempting real NLP.
 *
 *   "Call Rahul at 8pm"            -> today 20:00
 *   "Submit assignment tomorrow 10am" -> tomorrow 10:00
 *   "Pay electricity bill every month"
 *   "Drink water every 2 hours"
 */

export interface ParsedReminder {
  title: string;
  due: Date;
  recurrenceRule: string | null;
  matchedDate: boolean;
  matchedTime: boolean;
  matchedRecurrence: boolean;
}

const WEEKDAYS: Record<string, number> = {
  sunday: 0, sun: 0,
  monday: 1, mon: 1,
  tuesday: 2, tue: 2, tues: 2,
  wednesday: 3, wed: 3,
  thursday: 4, thu: 4, thurs: 4,
  friday: 5, fri: 5,
  saturday: 6, sat: 6,
};

const WEEKDAY_CODES = ["SU", "MO", "TU", "WE", "TH", "FR", "SA"];

class Stripper {
  private ranges: [number, number][] = [];

  add(start: number, end: number) {
    this.ranges.push([start, end]);
  }

  apply(text: string): string {
    if (!this.ranges.length) return text.trim();
    const sorted = [...this.ranges].sort((a, b) => a[0] - b[0]);
    let out = "";
    let cursor = 0;
    for (const [start, end] of sorted) {
      if (start < cursor) continue;
      out += text.slice(cursor, start);
      cursor = end;
    }
    out += text.slice(cursor);
    return out.replace(/\s{2,}/g, " ").trim().replace(/^[\s,.-]+|[\s,.-]+$/g, "");
  }
}

interface Match {
  start: number;
  end: number;
}

function find(text: string, regex: RegExp): (Match & RegExpExecArray) | null {
  const m = regex.exec(text);
  if (!m) return null;
  return Object.assign(m, { start: m.index, end: m.index + m[0].length });
}

export function parseReminder(input: string, now = new Date()): ParsedReminder {
  const text = input;
  const lower = text.toLowerCase();
  const strip = new Stripper();

  let due = new Date(now);
  due.setSeconds(0, 0);
  let matchedTime = false;
  let matchedDate = false;
  let recurrenceRule: string | null = null;

  // --- Recurrence ---------------------------------------------------------
  const everyWeekday = find(lower, /\bevery\s+week\s?day\b/);
  const everyDay = find(lower, /\bevery\s+day\b/);
  const everyWeek = find(lower, /\bevery\s+week\b/);
  const everyMonth = find(lower, /\bevery\s+month\b/);
  const everyN = find(lower, /\bevery\s+(\d+)\s+(hour|hours|minute|minutes|day|days|week|weeks|month|months)\b/);
  const everyNamed = find(lower, /\bevery\s+(sunday|monday|tuesday|wednesday|thursday|friday|saturday)\b/);

  if (everyWeekday) {
    recurrenceRule = "FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR";
    strip.add(everyWeekday.start, everyWeekday.end);
  } else if (everyN) {
    const n = Number(everyN[1]);
    const unit = everyN[2];
    const freq = unit.startsWith("hour")
      ? "HOURLY"
      : unit.startsWith("minute")
        ? "MINUTELY"
        : unit.startsWith("day")
          ? "DAILY"
          : unit.startsWith("week")
            ? "WEEKLY"
            : "MONTHLY";
    recurrenceRule = `FREQ=${freq};INTERVAL=${n}`;
    strip.add(everyN.start, everyN.end);
  } else if (everyNamed) {
    const code = WEEKDAY_CODES[WEEKDAYS[everyNamed[1]]];
    recurrenceRule = `FREQ=WEEKLY;BYDAY=${code}`;
    strip.add(everyNamed.start, everyNamed.end);
  } else if (everyDay) {
    recurrenceRule = "FREQ=DAILY";
    strip.add(everyDay.start, everyDay.end);
  } else if (everyWeek) {
    recurrenceRule = "FREQ=WEEKLY";
    strip.add(everyWeek.start, everyWeek.end);
  } else if (everyMonth) {
    recurrenceRule = "FREQ=MONTHLY";
    strip.add(everyMonth.start, everyMonth.end);
  }

  // --- Time ---------------------------------------------------------------
  let hour: number | null = null;
  let minute = 0;

  const ampm = find(lower, /\b(?:at\s+)?(\d{1,2})(?::(\d{2}))?\s*(am|pm)\b/);
  const clock = find(lower, /\b(?:at\s+)?(\d{1,2}):(\d{2})\b/);

  if (ampm) {
    hour = Number(ampm[1]) % 12;
    minute = ampm[2] ? Number(ampm[2]) : 0;
    if (ampm[3] === "pm") hour += 12;
    strip.add(ampm.start, ampm.end);
  } else if (clock) {
    hour = Number(clock[1]);
    minute = Number(clock[2]);
    strip.add(clock.start, clock.end);
  }

  // --- Date ---------------------------------------------------------------
  let dayOffset: number | null = null;
  const inN = find(lower, /\bin\s+(\d+)\s+(hour|hours|minute|minutes|day|days|week|weeks)\b/);
  const tonight = find(lower, /\btonight\b/);
  const tomorrow = find(lower, /\btomorrow\b/);
  const dayAfter = find(lower, /\b(day after tomorrow)\b/);
  const today = find(lower, /\btoday\b/);
  const weekday = find(
    lower,
    /\b(next\s+)?(sunday|monday|tuesday|wednesday|thursday|friday|saturday)\b/,
  );

  if (recurrenceRule && everyN) {
    // "every 2 hours" already sets the schedule; base it on now.
  }

  if (inN) {
    const n = Number(inN[1]);
    const unit = inN[2];
    if (unit.startsWith("minute")) due = new Date(now.getTime() + n * 60_000);
    else if (unit.startsWith("hour")) due = new Date(now.getTime() + n * 3_600_000);
    else if (unit.startsWith("week")) due = new Date(now.getTime() + n * 7 * 86_400_000);
    else due = new Date(now.getTime() + n * 86_400_000);
    matchedDate = true;
    matchedTime = true;
    strip.add(inN.start, inN.end);
  } else if (dayAfter) {
    dayOffset = 2;
    strip.add(dayAfter.start, dayAfter.end);
  } else if (tomorrow) {
    dayOffset = 1;
    strip.add(tomorrow.start, tomorrow.end);
  } else if (tonight) {
    dayOffset = 0;
    if (hour === null) hour = 20;
    strip.add(tonight.start, tonight.end);
  } else if (today) {
    dayOffset = 0;
    strip.add(today.start, today.end);
  } else if (weekday && !everyNamed) {
    const target = WEEKDAYS[weekday[2]];
    const isNext = Boolean(weekday[1]);
    let delta = (target - now.getDay() + 7) % 7;
    if (delta === 0) delta = 7;
    if (isNext && delta < 7) {
      // "next monday" means the Monday of next week when we are already past it.
      if (now.getDay() <= target) delta += 7;
    }
    dayOffset = delta;
    strip.add(weekday.start, weekday.end);
  }

  if (dayOffset !== null) {
    const base = new Date(now);
    base.setDate(base.getDate() + dayOffset);
    due = base;
    matchedDate = true;
  }

  if (hour !== null) {
    due.setHours(hour, minute, 0, 0);
    matchedTime = true;
    // A bare time in the past rolls to tomorrow.
    if (!matchedDate && due.getTime() <= now.getTime()) {
      due.setDate(due.getDate() + 1);
    }
  } else if (!matchedDate && recurrenceRule === null) {
    due = new Date(now.getTime() + 3_600_000);
  } else if (!matchedTime) {
    due.setSeconds(0, 0);
  }

  const title = strip.apply(text) || text.trim();
  return {
    title,
    due,
    recurrenceRule,
    matchedDate,
    matchedTime,
    matchedRecurrence: recurrenceRule !== null,
  };
}
