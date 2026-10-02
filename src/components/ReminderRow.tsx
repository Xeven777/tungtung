import {
  Check,
  Clock,
  PencilSimple,
  Repeat,
  Trash,
} from "@phosphor-icons/react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { formatDay, formatTime } from "@/lib/dates";
import { describeRule } from "@/lib/recurrence";
import type { Reminder } from "@/lib/types";

interface Props {
  reminder: Reminder;
  showDay?: boolean;
  onToggle: (reminder: Reminder) => void;
  onEdit: (reminder: Reminder) => void;
  onDelete: (reminder: Reminder) => void;
  onSnooze: (reminder: Reminder) => void;
}

export function ReminderRow({
  reminder,
  showDay,
  onToggle,
  onEdit,
  onDelete,
  onSnooze,
}: Props) {
  const done = reminder.status === "completed";
  const overdue = reminder.status === "overdue";
  const recurring = describeRule(reminder.recurrenceRule);

  return (
    <div className="group flex items-center gap-4 border-b py-3 last:border-b-0">
      <button
        type="button"
        aria-label={
          done ? `Reopen ${reminder.title}` : `Complete ${reminder.title}`
        }
        onClick={() => onToggle(reminder)}
        className={cn(
          "grid size-5 shrink-0 place-items-center rounded-full border-[1.5px] transition-colors ease-out",
          done
            ? "border-emerald-500 bg-emerald-500 text-white"
            : "text-transparent hover:border-primary",
        )}
      >
        <Check size={12} weight="bold" />
      </button>

      <div className="w-19 shrink-0 text-sm tabular-nums text-muted-foreground">
        {formatTime(reminder.dueAt)}
      </div>

      <div className="min-w-0 flex-1">
        <div
          className={cn(
            "truncate text-sm font-medium",
            done && "text-muted-foreground line-through",
          )}
        >
          {reminder.title}
        </div>
        <div className="mt-0.5 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
          {showDay ? <span>{formatDay(reminder.dueAt)}</span> : null}
          {recurring ? (
            <Badge variant="secondary" className="gap-1">
              <Repeat size={10} /> {recurring}
            </Badge>
          ) : null}
          {overdue ? <Badge variant="destructive">Overdue</Badge> : null}
          {reminder.notes ? (
            <span className="truncate">{reminder.notes}</span>
          ) : null}
        </div>
      </div>

      <div className="flex gap-0.5 opacity-0 transition-opacity duration-150 ease-out group-hover:opacity-100 group-focus-within:opacity-100">
        {!done ? (
          <Button
            variant="ghost"
            size="icon-sm"
            title="Snooze"
            aria-label="Snooze reminder"
            onClick={() => onSnooze(reminder)}
          >
            <Clock size={15} />
          </Button>
        ) : null}
        <Button
          variant="ghost"
          size="icon-sm"
          title="Edit"
          aria-label="Edit reminder"
          onClick={() => onEdit(reminder)}
        >
          <PencilSimple size={15} />
        </Button>
        <Button
          variant="ghost"
          size="icon-sm"
          title="Delete"
          aria-label="Delete reminder"
          onClick={() => onDelete(reminder)}
        >
          <Trash size={15} />
        </Button>
      </div>
    </div>
  );
}
