import { useEffect, useState } from "react";
import { Play } from "@phosphor-icons/react";

import { Page, PageHeader, Section } from "@/components/layout";
import { Row, SoundRow } from "@/components/SettingRow";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { api } from "@/lib/api";
import { formatClock } from "@/lib/dates";
import type { MicroBreakStatus } from "@/lib/types";
import { useStore } from "@/store";

export function Breaks() {
  const settings = useStore((s) => s.settings);
  const update = useStore((s) => s.updateSetting);
  const sounds = useStore((s) => s.sounds);
  const [status, setStatus] = useState<MicroBreakStatus | null>(null);

  // The interval is owned by Rust so it keeps running in the tray, which means
  // the countdown here has to be read back rather than derived.
  useEffect(() => {
    let alive = true;
    const poll = () =>
      api
        .microBreakStatus()
        .then((next) => {
          if (alive) setStatus(next);
        })
        .catch(() => undefined);
    void poll();
    const id = setInterval(poll, 15_000);
    return () => {
      alive = false;
      clearInterval(id);
    };
  }, []);

  const set = (key: string, value: string) => void update(key, value);
  const enabled = settings.microBreaksEnabled === "true";
  const remaining = enabled ? status?.remainingSeconds ?? null : null;

  return (
    <Page>
      <PageHeader title="Breaks" subtitle="Gentle nudges to look away from the screen" />

      <Card className="mb-10">
        <CardContent className="flex items-end justify-between gap-6 py-7">
          <div>
            <div className="text-xs font-medium tracking-wider text-muted-foreground uppercase">
              {enabled ? "Next break" : "Micro-breaks are off"}
            </div>
            <div className="mt-2.5 text-5xl leading-none font-medium tabular-nums tracking-tight">
              {remaining == null ? "—" : formatClock(remaining)}
            </div>
            <div className="mt-2.5 text-xs text-muted-foreground">
              {Number(settings.microWorkMinutes) || 50} min of work, then a{" "}
              {Number(settings.microBreakMinutes) || 5} min break
            </div>
          </div>
          <Button onClick={() => void api.microBreakTrigger()}>
            <Play size={15} data-icon="inline-start" /> Start now
          </Button>
        </CardContent>
      </Card>

      <Section title="Schedule">
        <Card>
          <CardContent className="divide-y">
            <Row
              label="Enable micro-breaks"
              hint="A nudge after each work interval"
              control={
                <Switch
                  checked={enabled}
                  onCheckedChange={(checked) => set("microBreaksEnabled", String(checked))}
                />
              }
            />
            <Row
              label="Work interval"
              hint="minutes"
              control={
                <Input
                  type="number"
                  min={5}
                  max={180}
                  className="w-20"
                  value={settings.microWorkMinutes}
                  onChange={(event) => set("microWorkMinutes", event.target.value)}
                />
              }
            />
            <Row
              label="Break duration"
              hint="minutes"
              control={
                <Input
                  type="number"
                  min={1}
                  max={60}
                  className="w-20"
                  value={settings.microBreakMinutes}
                  onChange={(event) => set("microBreakMinutes", event.target.value)}
                />
              }
            />
          </CardContent>
        </Card>
      </Section>

      <Section title="When a break is due">
        <Card>
          <CardContent className="divide-y">
            <Row
              label="Show break overlay"
              hint="Cover the screen with a countdown"
              control={
                <Switch
                  checked={settings.microBreakOverlay === "true"}
                  onCheckedChange={(checked) => set("microBreakOverlay", String(checked))}
                />
              }
            />
            <SoundRow
              label="Break sound"
              value={settings.microBreakSound}
              sounds={sounds}
              volume={Number(settings.volume)}
              onChange={(value) => set("microBreakSound", value)}
            />
            <Row
              label="Respect quiet hours"
              hint={`Silent ${settings.quietStart} – ${settings.quietEnd} while quiet hours are on`}
              control={
                <Switch
                  checked={settings.quietHoursEnabled === "true"}
                  onCheckedChange={(checked) => set("quietHoursEnabled", String(checked))}
                />
              }
            />
          </CardContent>
        </Card>
      </Section>
    </Page>
  );
}
