import { toast } from "sonner";
import { create } from "zustand";
import { api } from "./lib/api";
import { playSound } from "./lib/sounds";
import type {
  AppSettings,
  Diagnostics,
  FocusPhase,
  FocusStatus,
  HabitWithStats,
  HistoryEntry,
  Reminder,
  Sound,
  TodayView,
} from "./lib/types";

export type Route =
  | "today"
  | "reminders"
  | "focus"
  | "breaks"
  | "habits"
  | "history"
  | "settings";
export type ThemeMode = "system" | "light" | "dark";

export const DEFAULT_SETTINGS: AppSettings = {
  theme: "system",
  notificationsEnabled: "true",
  soundEnabled: "true",
  volume: "70",
  reminderSound: "builtin-soft",
  pomodoroSound: "builtin-bell",
  breakSound: "builtin-minimal",
  habitSound: "none",
  quietHoursEnabled: "false",
  quietStart: "23:00",
  quietEnd: "08:00",
  snoozeMinutes: "10",
  microBreaksEnabled: "true",
  microWorkMinutes: "50",
  microBreakMinutes: "5",
  microBreakOverlay: "true",
  microBreakSound: "builtin-minimal",
  closeToTray: "true",
  startMinimized: "false",
  launchAtStartup: "false",
  focusMinutes: "25",
  shortBreakMinutes: "5",
  longBreakMinutes: "15",
  sessionsBeforeLongBreak: "4",
  autoStartBreaks: "true",
  autoStartFocus: "false",
};

export interface Toast {
  title: string;
  body?: string;
  icon?: string;
  durationMs?: number;
  actions?: { label: string; run: () => void }[];
}

export interface MicroBreakState {
  open: boolean;
  remaining: number;
  planned: number;
}

interface FocusState {
  phase: FocusPhase;
  running: boolean;
  remaining: number;
  planned: number;
  sessionId: string | null;
  completedInCycle: number;
}

interface Store {
  route: Route;
  setRoute: (route: Route) => void;

  settings: AppSettings;
  diagnostics: Diagnostics | null;
  sounds: Sound[];

  today: TodayView | null;
  reminders: Reminder[];
  habits: HabitWithStats[];
  history: HistoryEntry[];
  activity: [string, number][];
  focus: FocusState;
  micro: MicroBreakState;

  quickAddOpen: boolean;
  quickAddSeed: string;
  paletteOpen: boolean;
  editing: Reminder | null;

  loadSettings: () => Promise<void>;
  updateSetting: (key: string, value: string) => Promise<void>;
  loadSounds: () => Promise<void>;
  refreshToday: () => Promise<void>;
  refreshReminders: (scope?: string, search?: string) => Promise<void>;
  refreshHabits: () => Promise<void>;
  refreshHistory: (days?: number) => Promise<void>;
  loadDiagnostics: () => Promise<void>;
  refreshAll: () => Promise<void>;

  openQuickAdd: (seed?: string) => void;
  closeQuickAdd: () => void;
  setPaletteOpen: (open: boolean) => void;
  setEditing: (reminder: Reminder | null) => void;

  pushToast: (toast: Toast) => void;

  focusStart: () => Promise<void>;
  focusPause: () => void;
  focusResume: () => void;
  focusToggle: (force?: boolean) => void;
  focusSkip: () => Promise<void>;
  focusReset: () => Promise<void>;
  applyFocusStatus: (status: FocusStatus) => void;
  fetchFocus: () => Promise<void>;

  startMicroBreak: (seconds?: number) => void;
  dismissMicroBreak: () => void;
  snoozeMicroBreak: (waitMinutes?: number) => void;
}

let focusDisplay: ReturnType<typeof setInterval> | null = null;
let microInterval: ReturnType<typeof setInterval> | null = null;

// Guards `refreshAll` against overlapping passes.
let refreshing = false;
let refreshQueued = false;

function minutes(setting: string, fallback: number): number {
  const n = Number(setting);
  return Number.isFinite(n) && n > 0 ? n : fallback;
}

/** Mirrors the Rust-side gate in `prefs.rs` so both sides agree. */
export function inQuietHours(settings: AppSettings, date = new Date()): boolean {
  if (settings.quietHoursEnabled !== "true") return false;
  const [sh, sm] = (settings.quietStart || "23:00").split(":").map(Number);
  const [eh, em] = (settings.quietEnd || "08:00").split(":").map(Number);
  const mins = date.getHours() * 60 + date.getMinutes();
  const start = sh * 60 + sm;
  const end = eh * 60 + em;
  return start <= end ? mins >= start && mins < end : mins >= start || mins < end;
}

export const useStore = create<Store>((set, get) => ({
  route: "today",
  setRoute: (route) => set({ route }),

  settings: DEFAULT_SETTINGS,
  diagnostics: null,
  sounds: [],

  today: null,
  reminders: [],
  habits: [],
  history: [],
  activity: [],
  focus: {
    phase: "idle",
    running: false,
    remaining: 0,
    planned: 0,
    sessionId: null,
    completedInCycle: 0,
  },
  micro: { open: false, remaining: 0, planned: 0 },

  quickAddOpen: false,
  quickAddSeed: "",
  paletteOpen: false,
  editing: null,

  loadSettings: async () => {
    const stored = await api.getSettings().catch(() => ({}) as AppSettings);
    const settings = { ...DEFAULT_SETTINGS, ...stored };
    set({ settings });
    applyTheme(settings.theme as ThemeMode);
  },

  updateSetting: async (key, value) => {
    const settings = { ...get().settings, [key]: value };
    set({ settings });
    if (key === "theme") applyTheme(value as ThemeMode);
    await api.setSetting(key, value).catch(() => undefined);
  },

  loadSounds: async () => {
    const sounds = await api.listSounds().catch(() => []);
    set({ sounds });
  },

  refreshToday: async () => {
    const today = await api.todayView().catch(() => null);
    if (today) set({ today, habits: today.habits });
  },

  refreshReminders: async (scope = "active", search = "") => {
    const reminders = await api
      .listReminders({ scope, search })
      .catch(() => [] as Reminder[]);
    set({ reminders });
  },

  refreshHabits: async () => {
    const habits = await api.listHabits().catch(() => []);
    set({ habits });
  },

  refreshHistory: async (days) => {
    const [history, activity] = await Promise.all([
      api.history(days).catch(() => []),
      api.activityCounts(14).catch(() => [] as [string, number][]),
    ]);
    set({ history, activity });
  },

  loadDiagnostics: async () => {
    const diagnostics = await api.diagnostics().catch(() => null);
    set({ diagnostics });
  },

  refreshAll: async () => {
    // Several callers land here for one event (a reminder emits both
    // `reminder-fired` and `reminders-changed`, and each action refreshes after
    // writing). Collapsing them keeps the UI from re-rendering twice in a row.
    // A request that arrives mid-flight is never dropped — it runs once the
    // current pass finishes, so the last write always wins.
    if (refreshing) {
      refreshQueued = true;
      return;
    }

    refreshing = true;
    try {
      do {
        refreshQueued = false;
        await Promise.all([
          get().refreshToday(),
          get().refreshReminders(),
          get().refreshHabits(),
          get().loadSounds(),
          get().fetchFocus(),
        ]);
      } while (refreshQueued);
    } finally {
      refreshing = false;
      refreshQueued = false;
    }
  },

  openQuickAdd: (seed = "") => set({ quickAddOpen: true, quickAddSeed: seed }),
  closeQuickAdd: () => set({ quickAddOpen: false, quickAddSeed: "" }),
  setPaletteOpen: (paletteOpen) => set({ paletteOpen }),
  setEditing: (editing) => set({ editing }),

  // Toasts are rendered by sonner; the store only shapes the payload. The first
  // action is the primary one and the second becomes sonner's cancel slot,
  // which is exactly the Done / Snooze pairing used across the app.
  // `icon` renders as a leading emoji badge; duration defaults keep
  // event toasts on screen longer than quiet confirmations.
  pushToast: ({ title, body, icon, durationMs, actions = [] }) => {
    const [primary, secondary] = actions;
    const label = icon ? `${icon} ${title}` : title;
    toast(label, {
      description: body,
      duration: durationMs ?? (actions.length ? 12_000 : 6_000),
      action: primary ? { label: primary.label, onClick: primary.run } : undefined,
      cancel: secondary ? { label: secondary.label, onClick: secondary.run } : undefined,
    });
  },

  focusStart: async () => {
    const status = await api.focusStart().catch(() => null);
    if (status) get().applyFocusStatus(status);
  },

  focusPause: () => {
    void api
      .focusPause()
      .then((status) => get().applyFocusStatus(status))
      .catch(() => undefined);
  },

  focusResume: () => {
    void api
      .focusResume()
      .then((status) => get().applyFocusStatus(status))
      .catch(() => undefined);
  },

  focusToggle: (force) => {
    void api
      .focusToggle(force)
      .then((status) => get().applyFocusStatus(status))
      .catch(() => undefined);
  },

  focusSkip: async () => {
    const status = await api.focusSkip().catch(() => null);
    if (status) get().applyFocusStatus(status);
  },

  focusReset: async () => {
    const status = await api.focusReset().catch(() => null);
    if (status) get().applyFocusStatus(status);
  },

  applyFocusStatus: (status) => {
    set({
      focus: {
        phase: status.phase,
        running: status.running,
        remaining: status.remaining,
        planned: status.planned,
        sessionId: status.sessionId,
        completedInCycle: status.completedInCycle,
      },
    });
    ensureFocusTicker(get, set);
  },

  fetchFocus: async () => {
    const status = await api.focusStatus().catch(() => null);
    if (status) get().applyFocusStatus(status);
  },

  // A micro break is just a countdown the UI owns; the scheduler decides when
  // it opens and restarts the work interval once it closes.
  startMicroBreak: (seconds) => {
    const planned = Math.max(
      1,
      Math.round(seconds ?? minutes(get().settings.microBreakMinutes, 5) * 60),
    );
    set({ micro: { open: true, remaining: planned, planned } });
    startMicroTicking(get, set);
  },

  dismissMicroBreak: () => {
    const wasOpen = get().micro.open;
    stopMicroTicking();
    set({ micro: { open: false, remaining: 0, planned: 0 } });
    if (wasOpen) void api.microBreakReset().catch(() => undefined);
  },

  snoozeMicroBreak: (waitMinutes) => {
    stopMicroTicking();
    set({ micro: { open: false, remaining: 0, planned: 0 } });
    void api
      .microBreakSnooze(Math.max(1, Math.round(waitMinutes ?? 10)))
      .catch(() => undefined);
  },
}));

// ---------------------------------------------------------------------------
// Focus display ticker
// ---------------------------------------------------------------------------

// The timer itself lives in Rust; this only interpolates the countdown for
// display while running. Every `focus-changed` event (and `fetchFocus`)
// re-syncs to the authoritative deadline, so drift never accumulates. When
// the countdown hits zero the backend has already advanced the phase (or is
// about to) — refetch to reconcile rather than guessing the transition here.
type Get = () => Store;
type Set = (partial: Partial<Store>) => void;

function stopFocusDisplay() {
  if (focusDisplay) {
    clearInterval(focusDisplay);
    focusDisplay = null;
  }
}

function ensureFocusTicker(get: Get, set: Set) {
  if (!get().focus.running) {
    stopFocusDisplay();
    return;
  }
  if (focusDisplay) return;
  focusDisplay = setInterval(() => {
    const focus = get().focus;
    if (!focus.running) {
      stopFocusDisplay();
      return;
    }
    if (focus.remaining > 0) {
      set({ focus: { ...focus, remaining: focus.remaining - 1 } });
      return;
    }
    void get().fetchFocus();
  }, 1000);
}

// ---------------------------------------------------------------------------
// Micro break countdown
// ---------------------------------------------------------------------------

function stopMicroTicking() {
  if (microInterval) {
    clearInterval(microInterval);
    microInterval = null;
  }
}

function startMicroTicking(get: Get, set: Set) {
  stopMicroTicking();
  microInterval = setInterval(() => {
    const micro = get().micro;
    if (!micro.open) return;
    const remaining = micro.remaining - 1;
    if (remaining > 0) {
      set({ micro: { ...micro, remaining } });
      return;
    }

    stopMicroTicking();
    set({ micro: { open: false, remaining: 0, planned: 0 } });
    const { settings, sounds, pushToast } = get();
    if (settings.soundEnabled === "true" && !inQuietHours(settings)) {
      const sound = sounds.find((item) => item.id === settings.breakSound);
      playSound(settings.breakSound, {
        volume: Number(settings.volume),
        filePath: sound?.filePath,
      });
    }
    pushToast({ title: "Break over", body: "Back to it when you're ready." });
    void api.microBreakReset().catch(() => undefined);
  }, 1000);
}

// ---------------------------------------------------------------------------
// Theming (brand primary is fixed orange in index.css)
// ---------------------------------------------------------------------------

export function applyTheme(mode: ThemeMode) {
  const root = document.documentElement;
  const systemDark = window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false;
  const resolved = mode === "system" ? (systemDark ? "dark" : "light") : mode;
  // shadcn/ui themes by toggling the `.dark` class.
  root.classList.toggle("dark", resolved === "dark");
  root.style.colorScheme = resolved;
}

/** Keep the resolved theme in sync when the OS theme changes. */
export function watchSystemTheme() {
  const mq = window.matchMedia?.("(prefers-color-scheme: dark)");
  if (!mq) return () => undefined;
  const handler = () => {
    if (useStore.getState().settings.theme === "system") applyTheme("system");
  };
  mq.addEventListener("change", handler);
  return () => mq.removeEventListener("change", handler);
}
