import { api } from "./api";
import { playSound } from "./sounds";
import { useStore } from "../store";
import type { Reminder } from "./types";

/** Shared reminder mutations plus the small UI side effects they trigger. */
export function useReminderActions() {
  const pushToast = useStore((s) => s.pushToast);
  const setEditing = useStore((s) => s.setEditing);
  const settings = useStore((s) => s.settings);
  const refresh = async () => {
    await useStore.getState().refreshAll();
  };

  return {
    async toggle(reminder: Reminder) {
      if (reminder.status === "completed") {
        await api.reopenReminder(reminder.id);
      } else {
        await api.completeReminder(reminder.id);
        if (settings.soundEnabled === "true") {
          // A quiet confirmation, not a notification sound.
          const sound = useStore.getState().sounds.find((item) => item.id === settings.reminderSound);
          playSound(settings.reminderSound, {
            volume: Number(settings.volume) * 0.4,
            filePath: sound?.filePath,
          });
        }
      }
      await refresh();
    },

    openEditor(reminder: Reminder) {
      setEditing(reminder);
    },

    async remove(reminder: Reminder) {
      await api.deleteReminder(reminder.id);
      await refresh();
      pushToast({
        title: "Reminder deleted",
        body: reminder.title,
        actions: [
          {
            label: "Undo",
            run: () => {
              void (async () => {
                await api.createReminder({
                  title: reminder.title,
                  notes: reminder.notes,
                  dueAt: reminder.dueAt,
                  timezone: reminder.timezone,
                  recurrenceRule: reminder.recurrenceRule,
                  soundId: reminder.soundId,
                  notificationEnabled: reminder.notificationEnabled,
                });
                await refresh();
              })();
            },
          },
        ],
      });
    },

    async snooze(reminder: Reminder) {
      const minutes = Number(settings.snoozeMinutes) || 10;
      await api.snoozeReminder(reminder.id, minutes);
      await refresh();
      pushToast({ title: `Snoozed ${minutes} min`, body: reminder.title });
    },

    async skip(reminder: Reminder) {
      await api.skipReminder(reminder.id);
      await refresh();
    },
  };
}

/** Habit completion toggle with today's optimistic refresh. */
export function useHabitActions() {
  return {
    async toggle(habitId: string, date?: string) {
      await api.toggleHabit(habitId, date);
      await useStore.getState().refreshHabits();
      await useStore.getState().refreshToday();
    },
    async archive(habitId: string) {
      await api.archiveHabit(habitId);
      await useStore.getState().refreshHabits();
    },
  };
}
