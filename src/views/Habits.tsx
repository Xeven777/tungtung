import { useState } from "react";
import { Check, Fire, Plus, Trash } from "@phosphor-icons/react";
import { EmptyState } from "@/components/EmptyState";
import { Page, PageHeader } from "@/components/layout";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Progress } from "@/components/ui/progress";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useHabitActions } from "@/lib/actions";
import { api } from "@/lib/api";
import { isSameDay } from "@/lib/dates";
import { cn } from "@/lib/utils";
import { useStore } from "@/store";
import type { HabitWithStats, ScheduleType } from "@/lib/types";

const ICONS = ["💧", "📖", "🏃", "🧘", "💪", "🎯", "🌱", "🧠", "☀️", "🛏️"];
const DOW = ["M", "T", "W", "T", "F", "S", "S"];

function todayKey(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(
    date.getDate(),
  ).padStart(2, "0")}`;
}

export function Habits() {
  const habits = useStore((s) => s.habits);
  const pushToast = useStore((s) => s.pushToast);
  const actions = useHabitActions();
  const [creating, setCreating] = useState(false);

  return (
    <Page>
      <PageHeader
        title="Habits"
        subtitle={`${habits.length} active`}
        actions={
          <Button onClick={() => setCreating(true)}>
            <Plus size={15} data-icon="inline-start" /> New habit
          </Button>
        }
      />

      {habits.length === 0 ? (
        <EmptyState
          title="No habits yet."
          body="Start with one small habit."
          action={
            <Button onClick={() => setCreating(true)}>
              <Plus size={14} data-icon="inline-start" /> Add habit
            </Button>
          }
        />
      ) : (
        habits.map((habit) => (
          <HabitCard
            key={habit.id}
            habit={habit}
            onToggle={(date) => void actions.toggle(habit.id, date)}
            onArchive={() => {
              void actions.archive(habit.id);
              pushToast({ title: "Habit archived", body: habit.name });
            }}
          />
        ))
      )}

      {creating ? (
        <HabitCreator
          onClose={() => setCreating(false)}
          onSaved={(name) => {
            pushToast({ title: "Habit created", body: name });
            setCreating(false);
          }}
        />
      ) : null}
    </Page>
  );
}

function HabitCard({
  habit,
  onToggle,
  onArchive,
}: {
  habit: HabitWithStats;
  onToggle: (date?: string) => void;
  onArchive: () => void;
}) {
  const today = new Date();
  const key = todayKey(today);
  const percent = habit.targetThisWeek
    ? Math.min(100, Math.round((habit.completedThisWeek / habit.targetThisWeek) * 100))
    : 0;

  return (
    <Card className="mb-4">
      <CardHeader className="flex-row items-center gap-3">
        <span className="grid size-8 place-items-center rounded-lg bg-muted text-base">
          {habit.icon || "•"}
        </span>
        <CardTitle className="flex-1">{habit.name}</CardTitle>
        <Badge variant="secondary">
          {habit.completedThisWeek}/{habit.targetThisWeek} this week
        </Badge>
        {habit.currentStreak > 0 ? (
          <Badge className="gap-1">
            <Fire size={10} /> {habit.currentStreak}
          </Badge>
        ) : null}
        <Button variant="ghost" size="icon-sm" aria-label="Archive habit" onClick={onArchive}>
          <Trash size={15} />
        </Button>
      </CardHeader>

      <CardContent className="flex flex-col gap-4">
        <div className="grid grid-cols-7 gap-1.5">
          {habit.weekDates.map((date, index) => {
            const parsed = new Date(`${date}T00:00:00`);
            const done = habit.weekCompleted.includes(date);
            return (
              <button
                key={date}
                type="button"
                aria-label={`${habit.name} on ${date}`}
                disabled={date > key}
                onClick={() => onToggle(date)}
                className="flex flex-col items-center gap-1 disabled:opacity-40"
              >
                <span className="text-[11px] text-muted-foreground">{DOW[index]}</span>
                <span
                  className={cn(
                    "grid size-6 place-items-center rounded-full border text-transparent",
                    done && "border-transparent bg-primary/15 text-primary",
                    isSameDay(parsed, today) && !done && "border-primary",
                  )}
                >
                  <Check size={12} weight="bold" />
                </span>
              </button>
            );
          })}
        </div>

        <Progress value={percent} />
      </CardContent>
    </Card>
  );
}

function HabitCreator({
  onClose,
  onSaved,
}: {
  onClose: () => void;
  onSaved: (name: string) => void;
}) {
  const [name, setName] = useState("");
  const [icon, setIcon] = useState(ICONS[0]);
  const [scheduleType, setScheduleType] = useState<ScheduleType>("daily");
  const [timesPerWeek, setTimesPerWeek] = useState(5);
  const [time, setTime] = useState("");

  async function save() {
    if (!name.trim()) return;
    const scheduleData =
      scheduleType === "times_per_week"
        ? JSON.stringify({ times: timesPerWeek })
        : scheduleType === "custom"
          ? JSON.stringify(["MO", "TU", "WE", "TH", "FR"])
          : JSON.stringify({});

    await api.createHabit({
      name: name.trim(),
      icon,
      color: null,
      scheduleType,
      scheduleData,
      reminderTime: time || null,
    });
    await useStore.getState().refreshHabits();
    await useStore.getState().refreshToday();
    onSaved(name.trim());
  }

  return (
    <Dialog open onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>New habit</DialogTitle>
        </DialogHeader>

        <div className="grid gap-4">
          <div className="grid gap-2">
            <Label htmlFor="habit-name">Name</Label>
            <Input
              id="habit-name"
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="Read, Exercise, Water…"
              autoFocus
            />
          </div>

          <div className="grid gap-2">
            <Label>Icon</Label>
            <div className="flex flex-wrap gap-1.5">
              {ICONS.map((value) => (
                <Button
                  key={value}
                  variant="outline"
                  size="icon"
                  onClick={() => setIcon(value)}
                  className={cn("text-base", icon === value && "border-primary")}
                >
                  {value}
                </Button>
              ))}
            </div>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="grid gap-2">
              <Label>Schedule</Label>
              <Select
                value={scheduleType}
                onValueChange={(value) => setScheduleType(value as ScheduleType)}
              >
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="daily">Every day</SelectItem>
                  <SelectItem value="weekdays">Weekdays</SelectItem>
                  <SelectItem value="custom">Mon–Fri</SelectItem>
                  <SelectItem value="times_per_week">X times per week</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="grid gap-2">
              <Label htmlFor="habit-time">Reminder time</Label>
              <Input
                id="habit-time"
                type="time"
                value={time}
                onChange={(event) => setTime(event.target.value)}
              />
            </div>
          </div>

          {scheduleType === "times_per_week" ? (
            <div className="grid gap-2">
              <Label>Times per week: {timesPerWeek}</Label>
              <input
                type="range"
                min={1}
                max={7}
                value={timesPerWeek}
                onChange={(event) => setTimesPerWeek(Number(event.target.value))}
                className="accent-primary"
              />
            </div>
          ) : null}
        </div>

        <DialogFooter>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={save} disabled={!name.trim()}>
            Create habit
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
