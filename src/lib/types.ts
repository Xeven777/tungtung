export type ReminderStatus =
  | "scheduled"
  | "completed"
  | "snoozed"
  | "dismissed"
  | "skipped"
  | "overdue";

export interface Reminder {
  id: string;
  title: string;
  notes: string | null;
  dueAt: string;
  timezone: string | null;
  recurrenceRule: string | null;
  status: ReminderStatus;
  soundId: string | null;
  notificationEnabled: boolean;
  createdAt: string;
  updatedAt: string;
  completedAt: string | null;
}

export interface ReminderInput {
  title: string;
  notes?: string | null;
  dueAt: string;
  timezone?: string | null;
  recurrenceRule?: string | null;
  soundId?: string | null;
  notificationEnabled?: boolean;
}

export type ScheduleType = "daily" | "weekdays" | "custom" | "times_per_week";

export interface Habit {
  id: string;
  name: string;
  icon: string | null;
  color: string | null;
  scheduleType: ScheduleType;
  scheduleData: string;
  reminderTime: string | null;
  createdAt: string;
  archivedAt: string | null;
}

export interface HabitWithStats extends Habit {
  completedThisWeek: number;
  targetThisWeek: number;
  currentStreak: number;
  bestStreak: number;
  completedToday: boolean;
  weekDates: string[];
  weekCompleted: string[];
}

export interface HabitInput {
  name: string;
  icon?: string | null;
  color?: string | null;
  scheduleType: ScheduleType;
  scheduleData: string;
  reminderTime?: string | null;
}

export interface FocusSession {
  id: string;
  startedAt: string;
  endedAt: string | null;
  plannedSeconds: number;
  actualSeconds: number | null;
  sessionType: "focus" | "short_break" | "long_break";
  completed: boolean;
}

export interface Sound {
  id: string;
  name: string;
  filePath: string | null;
  bundled: boolean;
  enabled: boolean;
}

export interface Diagnostics {
  platform: string;
  desktop: string;
  display: string;
  appVersion: string;
  notificationAvailable: boolean;
  trayAvailable: boolean;
  audioAvailable: boolean;
  dbPath: string;
}

export interface TodayView {
  reminders: Reminder[];
  overdue: Reminder[];
  habits: HabitWithStats[];
  focusSeconds: number;
}

export interface MicroBreakStatus {
  enabled: boolean;
  remainingSeconds: number | null;
}

export interface HistoryEntry {
  id: string;
  kind: "reminder" | "focus" | "habit";
  title: string;
  at: string;
  detail: string | null;
}

export interface ExportBundle {
  version: number;
  exportedAt: string;
  reminders: Reminder[];
  habits: Habit[];
  habitCompletions: {
    id: string;
    habitId: string;
    completedDate: string;
    completedAt: string;
  }[];
  focusSessions: FocusSession[];
  settings: { key: string; value: string }[];
}

export type AppSettings = Record<string, string>;

export interface SettingsDefaults {
  [key: string]: string;
}
