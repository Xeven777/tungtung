import { useEffect, useState } from "react";
import { Bell, Clock, Fire } from "@phosphor-icons/react";
import type { Icon } from "@phosphor-icons/react";
import { EmptyState } from "@/components/EmptyState";
import { Page, PageHeader, Section } from "@/components/layout";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { formatRelative } from "@/lib/dates";
import { useStore } from "@/store";

const FILTERS = [
  { value: 1, label: "Today" },
  { value: 7, label: "7 days" },
  { value: 30, label: "30 days" },
  { value: 0, label: "All time" },
];

const ICONS: Record<string, Icon> = {
  reminder: Bell,
  focus: Clock,
  habit: Fire,
};

export function History() {
  const history = useStore((s) => s.history);
  const activity = useStore((s) => s.activity);
  const refreshHistory = useStore((s) => s.refreshHistory);
  const [days, setDays] = useState(7);

  useEffect(() => {
    void refreshHistory(days || undefined);
  }, [days, refreshHistory]);

  const max = Math.max(1, ...activity.map(([, count]) => count));

  return (
    <Page>
      <PageHeader
        title="History"
        subtitle={`${history.length} events`}
        actions={
          <div className="flex rounded-lg border p-0.5">
            {FILTERS.map((filter) => (
              <Button
                key={filter.value}
                size="sm"
                variant="ghost"
                onClick={() => setDays(filter.value)}
                className={cn(
                  "h-7 rounded-md px-2.5 text-xs font-normal text-muted-foreground",
                  days === filter.value && "bg-muted font-medium text-foreground",
                )}
              >
                {filter.label}
              </Button>
            ))}
          </div>
        }
      />

      <Section title="Activity · 14 days">
        <Card className="[--card-spacing:0px]">
          <CardContent className="flex h-20 items-end gap-1 px-4 py-4">
            {activity.map(([date, count]) => (
              <div
                key={date}
                title={`${date}: ${count}`}
                className={cn(
                  "flex-1 rounded-sm",
                  count === 0 ? "h-1 bg-muted" : "bg-primary/70",
                )}
                style={count === 0 ? undefined : { height: `${Math.max(12, (count / max) * 100)}%` }}
              />
            ))}
          </CardContent>
        </Card>
      </Section>

      <Section title="Timeline" meta={history.length}>
        {history.length === 0 ? (
          <EmptyState
            title="Nothing here yet."
            body="Completed reminders, focus sessions and habits show up here."
          />
        ) : (
          <div className="flex flex-col">
            {history.map((entry) => {
              const EntryIcon = ICONS[entry.kind] ?? Bell;
              return (
                <div
                  key={`${entry.kind}-${entry.id}`}
                  className="flex items-center gap-4 border-b py-3 last:border-b-0"
                >
                  <span className="w-20 shrink-0 text-xs text-muted-foreground">
                    {formatRelative(entry.at)}
                  </span>
                  <span className="text-muted-foreground">
                    <EntryIcon size={15} />
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-sm font-medium">{entry.title}</div>
                    {entry.detail ? (
                      <div className="text-xs text-muted-foreground">{entry.detail}</div>
                    ) : null}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </Section>
    </Page>
  );
}
