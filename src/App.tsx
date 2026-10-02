import { Suspense, lazy, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";

import { CommandPalette } from "@/components/CommandPalette";
import { MicroBreakOverlay } from "@/components/MicroBreakOverlay";
import { QuickAdd } from "@/components/QuickAdd";
import { ReminderEditor } from "@/components/ReminderEditor";
import { Sidebar } from "@/components/Sidebar";
import { TitleBar } from "@/components/TitleBar";
import { Toaster } from "@/components/ui/sonner";
import { api } from "@/lib/api";
import { playSound } from "@/lib/sounds";
import type { Reminder } from "@/lib/types";
import { inQuietHours, useStore, watchSystemTheme, type Route } from "@/store";
import { Today } from "@/views/Today";

const Reminders = lazy(() =>
  import("@/views/Reminders").then((m) => ({ default: m.Reminders })),
);
const Focus = lazy(() =>
  import("@/views/Focus").then((m) => ({ default: m.Focus })),
);
const Breaks = lazy(() =>
  import("@/views/Breaks").then((m) => ({ default: m.Breaks })),
);
const Habits = lazy(() =>
  import("@/views/Habits").then((m) => ({ default: m.Habits })),
);
const History = lazy(() =>
  import("@/views/History").then((m) => ({ default: m.History })),
);
const Settings = lazy(() =>
  import("@/views/Settings").then((m) => ({ default: m.Settings })),
);

const ROUTE_KEYS: Record<string, Route> = {
  "1": "today",
  "2": "reminders",
  "3": "focus",
  "4": "breaks",
  "5": "habits",
};

// Bootstrapping must happen once per app lifetime, not once per mount:
// StrictMode mounts effects twice in development, so guard against double init.
let bootstrapped = false;

export default function App() {
  const route = useStore((s) => s.route);

  // Initial load.
  // Note: desktop notifications via notify-rust need no permission handshake —
  // Linux daemons grant implicitly — so there is no permission request here.
  useEffect(() => {
    if (bootstrapped) return;
    bootstrapped = true;

    void (async () => {
      await useStore.getState().loadSettings();
      await useStore.getState().refreshAll();
      await useStore.getState().loadDiagnostics();
    })();
  }, []);

  // Keep the OS-driven theme in sync.
  useEffect(() => watchSystemTheme(), []);

  // Backend events.
  useEffect(() => {
    // `listen` resolves asynchronously, so a listener registered just before the
    // effect is torn down (which happens on StrictMode's double mount) would
    // otherwise never be removed — and every event would fire twice.
    let disposed = false;
    const unlisteners: (() => void)[] = [];
    const track = (pending: Promise<() => void>) => {
      void pending.then((unlisten) => {
        if (disposed) unlisten();
        else unlisteners.push(unlisten);
      });
    };

    track(
      listen<Reminder>("reminder-fired", (event) => {
        const reminder = event.payload;
        const state = useStore.getState();
        const sound = state.sounds.find(
          (item) => item.id === state.settings.reminderSound,
        );

        // Quiet hours mute the app's own sound too, not just the OS notification.
        if (
          state.settings.soundEnabled === "true" &&
          !inQuietHours(state.settings)
        ) {
          playSound(state.settings.reminderSound, {
            volume: Number(state.settings.volume),
            filePath: sound?.filePath,
          });
        }

        state.pushToast({
          title: reminder.title,
          body: reminder.notes?.trim() ? reminder.notes : "It's time.",
          icon: "⏰",
          durationMs: 15_000,
          actions: [
            {
              label: "Done",
              run: () => {
                void api
                  .completeReminder(reminder.id)
                  .then(() => state.refreshAll());
              },
            },
            {
              label: `Snooze ${state.settings.snoozeMinutes}m`,
              run: () => {
                void api
                  .snoozeReminder(
                    reminder.id,
                    Number(state.settings.snoozeMinutes) || 10,
                  )
                  .then(() => state.refreshAll());
              },
            },
          ],
        });
        void state.refreshAll();
      }),
    );

    track(
      listen<{ id: string; name: string; icon: string | null }>(
        "habit-reminder-fired",
        (event) => {
          const habit = event.payload;
          const state = useStore.getState();
          const sound = state.sounds.find(
            (item) => item.id === state.settings.habitSound,
          );

          if (
            state.settings.soundEnabled === "true" &&
            state.settings.habitSound !== "none" &&
            !inQuietHours(state.settings)
          ) {
            playSound(state.settings.habitSound, {
              volume: Number(state.settings.volume),
              filePath: sound?.filePath,
            });
          }

          state.pushToast({
            title: `${habit.icon ? `${habit.icon} ` : ""}Time for ${habit.name}`,
            body: "Habit reminder",
            actions: [
              {
                label: "Mark done",
                run: () => {
                  void api.toggleHabit(habit.id).then(() => state.refreshAll());
                },
              },
            ],
          });
        },
      ),
    );

    track(
      listen<{ breakSeconds?: number }>("micro-break-due", (event) => {
        const state = useStore.getState();
        const breakSeconds =
          event.payload?.breakSeconds ??
          (Number(state.settings.microBreakMinutes) || 5) * 60;
        const sound = state.sounds.find(
          (item) => item.id === state.settings.microBreakSound,
        );

        if (
          state.settings.soundEnabled === "true" &&
          state.settings.microBreakSound !== "none" &&
          !inQuietHours(state.settings)
        ) {
          playSound(state.settings.microBreakSound, {
            volume: Number(state.settings.volume),
            filePath: sound?.filePath,
          });
        }

        if (state.settings.microBreakOverlay === "true") {
          state.startMicroBreak(breakSeconds);
          return;
        }

        // Without the overlay the nudge has to offer the break instead.
        state.pushToast({
          title: "Time for a break",
          body: "Look away from the screen for a few minutes.",
          actions: [
            {
              label: "Start break",
              run: () => useStore.getState().startMicroBreak(breakSeconds),
            },
            {
              label: "Snooze",
              run: () =>
                useStore
                  .getState()
                  .snoozeMicroBreak(Number(state.settings.snoozeMinutes) || 10),
            },
          ],
        });
      }),
    );

    track(
      listen("reminders-changed", () => {
        void useStore.getState().refreshAll();
      }),
    );

    track(
      listen("quick-add", () => {
        useStore.getState().openQuickAdd();
      }),
    );

    track(
      listen<boolean>("focus-toggle", (event) => {
        useStore.getState().focusToggle(event.payload);
      }),
    );

    track(
      listen<string>("navigate", (event) => {
        useStore.getState().setRoute(event.payload as Route);
      }),
    );

    return () => {
      disposed = true;
      unlisteners.forEach((un) => un());
    };
  }, []);

  // Local keyboard shortcuts. The global quick-add shortcut lives in Rust.
  // Note: quick add and the command palette intentionally have no entrance
  // animation — they are keyboard-initiated and used many times a day.
  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      const mod = event.ctrlKey || event.metaKey;
      if (mod && event.key.toLowerCase() === "k") {
        event.preventDefault();
        useStore.getState().setPaletteOpen(true);
        return;
      }
      if (mod && ROUTE_KEYS[event.key]) {
        event.preventDefault();
        useStore.getState().setRoute(ROUTE_KEYS[event.key]);
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  return (
    <div className="flex h-screen flex-col overflow-hidden border rounded-xl bg-background">
      <TitleBar />

      <div className="grid min-h-0 flex-1 grid-cols-[216px_1fr] max-[860px]:grid-cols-[64px_1fr]">
        <Sidebar />
        <main id="main" className="min-h-0 overflow-y-auto">
          <Suspense
            fallback={
              <div className="animate-pulse space-y-3 p-6">
                <div className="h-7 w-40 rounded-lg bg-muted" />
                <div className="h-24 rounded-xl bg-muted/70" />
                <div className="h-24 rounded-xl bg-muted/70" />
              </div>
            }
          >
            {route === "today" ? <Today /> : null}
            {route === "reminders" ? <Reminders /> : null}
            {route === "focus" ? <Focus /> : null}
            {route === "breaks" ? <Breaks /> : null}
            {route === "habits" ? <Habits /> : null}
            {route === "history" ? <History /> : null}
            {route === "settings" ? <Settings /> : null}
          </Suspense>
        </main>
      </div>

      <MicroBreakOverlay />
      <QuickAdd />
      <ReminderEditor />
      <CommandPalette />
      <Toaster />
    </div>
  );
}
