import { useEffect, useMemo, useRef, useState } from "react";
import { Bell, CalendarBlank, Clock, Repeat } from "@phosphor-icons/react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { api } from "@/lib/api";
import {
  formatDay,
  formatTime,
  fromLocalInputValue,
  toIsoUtc,
  toLocalInputValue,
} from "@/lib/dates";
import { parseReminder } from "@/lib/parse";
import { RECURRENCE_OPTIONS } from "@/lib/recurrence";
import { useStore } from "@/store";

const NONE = "none";

export function QuickAdd() {
  const open = useStore((s) => s.quickAddOpen);
  const seed = useStore((s) => s.quickAddSeed);
  const close = useStore((s) => s.closeQuickAdd);
  const pushToast = useStore((s) => s.pushToast);
  const settings = useStore((s) => s.settings);

  const inputRef = useRef<HTMLInputElement>(null);
  const [raw, setRaw] = useState("");
  const [rule, setRule] = useState<string | null>(null);
  const [when, setWhen] = useState("");
  const [saving, setSaving] = useState(false);

  const parsed = useMemo(() => parseReminder(raw), [raw]);

  useEffect(() => {
    if (!open) return;
    setRaw(seed);
    setRule(null);
    setWhen("");
    setSaving(false);
    const timer = setTimeout(() => inputRef.current?.focus(), 30);
    return () => clearTimeout(timer);
  }, [open, seed]);

  const effectiveRule = rule ?? parsed.recurrenceRule;
  const effectiveDue = when ? fromLocalInputValue(when) : parsed.due;

  // Date and time are separate fields so either can be edited without touching
  // the other; `when` stays the single source of truth underneath.
  const [datePart, timePart = "00:00"] = (
    when || toLocalInputValue(effectiveDue)
  ).split("T");
  const setDatePart = (value: string) => setWhen(`${value}T${timePart}`);
  const setTimePart = (value: string) => setWhen(`${datePart}T${value}`);

  async function submit() {
    const title = parsed.title.trim();
    if (!title || saving) return;
    setSaving(true);
    try {
      await api.createReminder({
        title,
        dueAt: toIsoUtc(effectiveDue),
        timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
        recurrenceRule: effectiveRule,
        soundId: settings.reminderSound,
        notificationEnabled: settings.notificationsEnabled === "true",
      });
      await useStore.getState().refreshAll();
      pushToast({
        title: "Reminder created",
        body: `${title} · ${formatDay(effectiveDue.toISOString())}`,
      });
      close();
    } catch (err) {
      pushToast({ title: "Could not create reminder", body: String(err) });
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={(next) => !next && close()}>
      <DialogContent
        showCloseButton={false}
        // Keyboard-initiated (Ctrl+Shift+Space) and used many times a day, so
        // there is deliberately no entrance animation.
        className="gap-0 overflow-hidden p-0 duration-0 sm:max-w-lg data-open:animate-none"
        onKeyDown={(event) => {
          if (event.key === "Enter" && !event.shiftKey) {
            event.preventDefault();
            void submit();
          }
        }}
      >
        <DialogHeader className="sr-only">
          <DialogTitle>Quick add reminder</DialogTitle>
        </DialogHeader>

        <div className="flex items-center gap-3 px-6 pt-6 pb-5">
          <Bell size={18} className="shrink-0 text-muted-foreground" />
          <input
            ref={inputRef}
            value={raw}
            onChange={(event) => {
              setRaw(event.target.value);
              setRule(null);
              setWhen("");
            }}
            placeholder="What do you need to remember?"
            className="w-full bg-transparent text-lg outline-none placeholder:text-muted-foreground"
          />
        </div>

        <div className="grid grid-cols-2 gap-4 border-t bg-muted/40 px-6 py-5 sm:grid-cols-3">
          <div className="grid gap-2">
            <Label htmlFor="quick-add-date" className="text-xs">
              <CalendarBlank size={12} /> Date
            </Label>
            <Input
              id="quick-add-date"
              type="date"
              className="bg-background"
              value={datePart}
              onChange={(event) => setDatePart(event.target.value)}
            />
          </div>

          <div className="grid gap-2">
            <Label htmlFor="quick-add-time" className="text-xs">
              <Clock size={12} /> Time
            </Label>
            <Input
              id="quick-add-time"
              type="time"
              className="bg-background"
              value={timePart}
              onChange={(event) => setTimePart(event.target.value)}
            />
          </div>

          <div className="grid gap-2">
            <Label className="text-xs">
              <Repeat size={12} /> Repeat
            </Label>
            <Select
              value={effectiveRule ?? NONE}
              onValueChange={(value) => setRule(value === NONE ? "" : value)}
            >
              <SelectTrigger className="w-full bg-background">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {RECURRENCE_OPTIONS.map((option) => (
                  <SelectItem
                    key={option.value || NONE}
                    value={option.value || NONE}
                  >
                    {option.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>

        <DialogFooter className="items-center justify-between sm:justify-between px-10 pb-8">
          <span className="hidden text-xs text-muted-foreground sm:block">
            {formatDay(effectiveDue.toISOString())} ·{" "}
            {formatTime(effectiveDue.toISOString())}
            {effectiveRule ? " · repeats" : ""}
          </span>
          <div className="ml-auto flex gap-2">
            <Button variant="ghost" onClick={close}>
              Cancel
            </Button>
            <Button onClick={submit} disabled={!parsed.title.trim() || saving}>
              Create
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
