import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  Diagnostics,
  ExportBundle,
  FocusSession,
  Habit,
  HabitInput,
  HabitWithStats,
  HistoryEntry,
  MicroBreakStatus,
  PendingAction,
  Reminder,
  ReminderInput,
  Sound,
  TodayView,
} from "./types";

export const api = {
  listReminders: (filter?: { scope?: string; search?: string }) =>
    invoke<Reminder[]>("list_reminders", { filter: filter ?? null }),

  createReminder: (input: ReminderInput) =>
    invoke<Reminder>("create_reminder", { input }),

  updateReminder: (id: string, input: ReminderInput) =>
    invoke<Reminder>("update_reminder", { id, input }),

  completeReminder: (id: string) => invoke<Reminder | null>("complete_reminder", { id }),

  reopenReminder: (id: string) => invoke<Reminder | null>("reopen_reminder", { id }),

  skipReminder: (id: string) => invoke<Reminder | null>("skip_reminder", { id }),

  snoozeReminder: (id: string, minutes: number) =>
    invoke<Reminder | null>("snooze_reminder", { id, minutes }),

  deleteReminder: (id: string) => invoke<void>("delete_reminder", { id }),

  listHabits: () => invoke<HabitWithStats[]>("list_habits"),

  createHabit: (input: HabitInput) => invoke<Habit>("create_habit", { input }),

  updateHabit: (id: string, input: HabitInput) =>
    invoke<Habit>("update_habit", { id, input }),

  archiveHabit: (id: string) => invoke<void>("archive_habit", { id }),

  toggleHabit: (habitId: string, date?: string) =>
    invoke<boolean>("toggle_habit", { habitId, date: date ?? null }),

  startFocusSession: (plannedSeconds: number, sessionType: string) =>
    invoke<FocusSession>("start_focus_session", {
      input: { plannedSeconds, sessionType },
    }),

  endFocusSession: (id: string, actualSeconds: number, completed: boolean) =>
    invoke<FocusSession | null>("end_focus_session", { id, actualSeconds, completed }),

  listFocusSessions: (limit = 50) =>
    invoke<FocusSession[]>("list_focus_sessions", { limit }),

  microBreakStatus: () => invoke<MicroBreakStatus>("micro_break_status"),

  microBreakReset: () => invoke<void>("micro_break_reset"),

  microBreakTrigger: () => invoke<void>("micro_break_trigger"),

  microBreakSnooze: (minutes: number) =>
    invoke<void>("micro_break_snooze", { minutes }),

  listSounds: () => invoke<Sound[]>("list_sounds"),

  addSound: (name: string, filePath: string) =>
    invoke<Sound>("add_sound", { name, filePath }),

  importSound: (name: string, sourcePath: string) =>
    invoke<Sound>("import_sound", { name, sourcePath }),

  deleteSound: (id: string) => invoke<void>("delete_sound", { id }),

  getSettings: () => invoke<AppSettings>("get_settings"),

  setSetting: (key: string, value: string) =>
    invoke<void>("set_setting", { key, value }),

  todayView: () => invoke<TodayView>("today_view"),

  history: (days?: number) => invoke<HistoryEntry[]>("history", { days: days ?? null }),

  activityCounts: (days = 14) =>
    invoke<[string, number][]>("activity_counts", { days }),

  diagnostics: () => invoke<Diagnostics>("diagnostics"),

  exportData: () => invoke<ExportBundle>("export_data"),

  importData: (bundle: ExportBundle) => invoke<void>("import_data", { bundle }),

  parseExport: (raw: string) => invoke<ExportBundle>("parse_export", { raw }),

  clearAllData: () => invoke<void>("clear_all_data"),

  /**
   * Tell the backend the UI is mounted with its listeners registered. Returns
   * an action requested while the window did not exist (e.g. a tray click that
   * recreated the window), so the caller can apply it.
   */
  uiReady: () => invoke<PendingAction | null>("ui_ready"),
};
