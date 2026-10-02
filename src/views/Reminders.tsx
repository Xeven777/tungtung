import { useEffect, useState } from "react";
import { MagnifyingGlass, Plus } from "@phosphor-icons/react";
import { EmptyState } from "@/components/EmptyState";
import { Page, PageHeader } from "@/components/layout";
import { ReminderRow } from "@/components/ReminderRow";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useReminderActions } from "@/lib/actions";
import { cn } from "@/lib/utils";
import { useStore } from "@/store";

const SCOPES = [
  { value: "active", label: "Active" },
  { value: "overdue", label: "Overdue" },
  { value: "completed", label: "Completed" },
  { value: "all", label: "All" },
];

export function Reminders() {
  const reminders = useStore((s) => s.reminders);
  const refreshReminders = useStore((s) => s.refreshReminders);
  const openQuickAdd = useStore((s) => s.openQuickAdd);
  const actions = useReminderActions();

  const [scope, setScope] = useState("active");
  const [search, setSearch] = useState("");

  useEffect(() => {
    const timer = setTimeout(() => void refreshReminders(scope, search), 160);
    return () => clearTimeout(timer);
  }, [scope, search, refreshReminders]);

  return (
    <Page>
      <PageHeader
        title="Reminders"
        subtitle={`${reminders.length} shown`}
        actions={
          <Button onClick={() => openQuickAdd()}>
            <Plus size={15} data-icon="inline-start" /> New
          </Button>
        }
      />

      <div className="mb-5 flex items-center gap-3">
        <div className="flex rounded-lg border p-0.5">
          {SCOPES.map((item) => (
            <Button
              key={item.value}
              size="sm"
              variant="ghost"
              onClick={() => setScope(item.value)}
              className={cn(
                "h-7 rounded-md px-2.5 text-xs font-normal text-muted-foreground",
                scope === item.value && "bg-muted font-medium text-foreground",
              )}
            >
              {item.label}
            </Button>
          ))}
        </div>

        <div className="relative flex-1">
          <MagnifyingGlass
            size={15}
            className="absolute top-1/2 left-3 -translate-y-1/2 text-muted-foreground"
          />
          <Input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Search reminders…"
            className="pl-9"
          />
        </div>
      </div>

      {reminders.length === 0 ? (
        <EmptyState
          title="No reminders here."
          body="Create your first reminder to get started."
          action={
            <Button onClick={() => openQuickAdd()}>
              <Plus size={14} data-icon="inline-start" /> Add reminder
            </Button>
          }
        />
      ) : (
        <div className="flex flex-col">
          {reminders.map((reminder) => (
            <ReminderRow
              key={reminder.id}
              reminder={reminder}
              showDay
              onToggle={actions.toggle}
              onEdit={actions.openEditor}
              onDelete={actions.remove}
              onSnooze={actions.snooze}
            />
          ))}
        </div>
      )}
    </Page>
  );
}
