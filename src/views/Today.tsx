import { useMemo } from "react";
import { Check, Pause, Play, Plus } from "@phosphor-icons/react";
import { EmptyState } from "@/components/EmptyState";
import { Page, PageHeader, Section } from "@/components/layout";
import { ReminderRow } from "@/components/ReminderRow";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { useHabitActions, useReminderActions } from "@/lib/actions";
import { formatClock, greeting, isSameDay, longDate } from "@/lib/dates";
import { cn } from "@/lib/utils";
import { useStore } from "@/store";

export function Today() {
  const today = useStore((s) => s.today);
  const reminders = useStore((s) => s.reminders);
  const focus = useStore((s) => s.focus);
  const setRoute = useStore((s) => s.setRoute);
  const openQuickAdd = useStore((s) => s.openQuickAdd);
  const focusToggle = useStore((s) => s.focusToggle);
  const actions = useReminderActions();
  const habitActions = useHabitActions();

  const tomorrow = useMemo(() => {
    const date = new Date();
    date.setDate(date.getDate() + 1);
    return date;
  }, [today]);

  const upcoming = today?.reminders ?? [];
  const overdue = today?.overdue ?? [];
  const habits = today?.habits ?? [];
  const tomorrowItems = reminders.filter((reminder) =>
    isSameDay(new Date(reminder.dueAt), tomorrow),
  );
  const incomplete = overdue.length + upcoming.length;

  return (
    <Page>
      <PageHeader
        title={greeting()}
        subtitle={longDate()}
        actions={
          <Button onClick={() => openQuickAdd()}>
            <Plus size={15} data-icon="inline-start" /> Add reminder
          </Button>
        }
      />

      <Section
        title="Today"
        meta={incomplete === 0 ? "You're clear" : `${incomplete} left`}
      >
        {overdue.length > 0 ? (
          <div className="mb-3">
            {overdue.map((reminder) => (
              <ReminderRow
                key={reminder.id}
                reminder={reminder}
                onToggle={actions.toggle}
                onEdit={actions.openEditor}
                onDelete={actions.remove}
                onSnooze={actions.snooze}
              />
            ))}
          </div>
        ) : null}

        {upcoming.length === 0 && overdue.length === 0 ? (
          <EmptyState
            title="You're clear for today."
            body="Nothing scheduled. Enjoy the quiet."
            action={
              <Button variant="outline" onClick={() => openQuickAdd()}>
                <Plus size={14} data-icon="inline-start" /> Add reminder
              </Button>
            }
          />
        ) : (
          upcoming.map((reminder) => (
            <ReminderRow
              key={reminder.id}
              reminder={reminder}
              onToggle={actions.toggle}
              onEdit={actions.openEditor}
              onDelete={actions.remove}
              onSnooze={actions.snooze}
            />
          ))
        )}
      </Section>

      {tomorrowItems.length > 0 ? (
        <Section title="Tomorrow" meta={tomorrowItems.length}>
          {tomorrowItems.map((reminder) => (
            <ReminderRow
              key={reminder.id}
              reminder={reminder}
              onToggle={actions.toggle}
              onEdit={actions.openEditor}
              onDelete={actions.remove}
              onSnooze={actions.snooze}
            />
          ))}
        </Section>
      ) : null}

      <Section
        title="Focus"
        action={
          <Button variant="ghost" size="sm" onClick={() => setRoute("focus")}>
            Open
          </Button>
        }
      >
        <Card>
          <CardContent className="flex items-center justify-between gap-4">
            <div>
              <div className="text-3xl font-medium tabular-nums">
                {focus.phase === "idle"
                  ? `${Math.round((today?.focusSeconds ?? 0) / 60)}m today`
                  : formatClock(focus.remaining)}
              </div>
              <div className="text-sm text-muted-foreground">
                {focus.phase === "idle"
                  ? "Ready when you are"
                  : focus.phase.replace("_", " ")}
              </div>
            </div>
            <Button onClick={() => focusToggle()}>
              {focus.running ? (
                <Pause size={14} data-icon="inline-start" />
              ) : (
                <Play size={14} data-icon="inline-start" />
              )}
              {focus.running
                ? "Pause"
                : focus.phase === "idle"
                  ? "Start focus"
                  : "Resume"}
            </Button>
          </CardContent>
        </Card>
      </Section>

      <Section
        title="Habits"
        meta={`${habits.filter((habit) => habit.completedToday).length}/${habits.length}`}
      >
        {habits.length === 0 ? (
          <EmptyState
            title="No habits yet."
            body="Start with one small habit."
            action={
              <Button variant="outline" onClick={() => setRoute("habits")}>
                <Plus size={14} data-icon="inline-start" /> Add a habit
              </Button>
            }
          />
        ) : (
          <div className="flex flex-col">
            {habits.slice(0, 6).map((habit) => (
              <button
                key={habit.id}
                type="button"
                onClick={() => habitActions.toggle(habit.id)}
                className="flex w-full items-center gap-4 border-b py-3 text-left last:border-b-0"
              >
                <span
                  className={cn(
                    "grid size-5 shrink-0 place-items-center rounded-full border-[1.5px] transition-colors ease-out",
                    habit.completedToday
                      ? "border-emerald-500 bg-emerald-500 text-white"
                      : "text-transparent hover:border-primary",
                  )}
                >
                  <Check size={12} weight="bold" />
                </span>
                <span className="text-lg leading-none">
                  {habit.icon || "•"}
                </span>
                <span className="flex-1 truncate text-sm font-medium">
                  {habit.name}
                </span>
                <Badge variant="secondary">
                  {habit.currentStreak} day streak
                </Badge>
              </button>
            ))}
          </div>
        )}
      </Section>
    </Page>
  );
}
