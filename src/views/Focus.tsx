import { ArrowCounterClockwise, Pause, Play, SkipForward } from "@phosphor-icons/react";
import { Page, PageHeader, Section } from "@/components/layout";
import {
  Card,
  CardContent,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";
import { formatClock } from "@/lib/dates";
import { useStore } from "@/store";

const PHASE_LABEL: Record<string, string> = {
  idle: "Ready",
  focus: "Working",
  short_break: "Short break",
  long_break: "Long break",
};

export function Focus() {
  const focus = useStore((s) => s.focus);
  const settings = useStore((s) => s.settings);
  const updateSetting = useStore((s) => s.updateSetting);
  const start = useStore((s) => s.focusStart);
  const resume = useStore((s) => s.focusResume);
  const pause = useStore((s) => s.focusPause);
  const skip = useStore((s) => s.focusSkip);
  const reset = useStore((s) => s.focusReset);

  const idleMinutes = Number(settings.focusMinutes) || 25;
  const display = focus.phase === "idle" ? idleMinutes * 60 : focus.remaining;
  const dots = Number(settings.sessionsBeforeLongBreak) || 4;

  return (
    <Page>
      <PageHeader title="Focus" subtitle={PHASE_LABEL[focus.phase]} />

      <Card className="mb-8 [--card-spacing:0px]">
        <CardContent className="flex flex-col items-center gap-6 py-10">
          <div className="text-7xl leading-none font-medium tabular-nums tracking-tight">
            {formatClock(display)}
          </div>
          <div className="text-sm text-muted-foreground">{PHASE_LABEL[focus.phase]}</div>

          <div className="flex gap-2" aria-label="Sessions in cycle">
            {Array.from({ length: dots }).map((_, index) => (
              <span
                key={index}
                className={cn(
                  "size-2 rounded-full border",
                  index < focus.completedInCycle ? "border-primary bg-primary" : "bg-muted",
                )}
              />
            ))}
          </div>

          <div className="flex gap-2">
            {focus.phase === "idle" ? (
              <Button onClick={() => void start()}>
                <Play size={15} data-icon="inline-start" /> Start focus
              </Button>
            ) : focus.running ? (
              <Button onClick={pause}>
                <Pause size={15} data-icon="inline-start" /> Pause
              </Button>
            ) : (
              <Button onClick={resume}>
                <Play size={15} data-icon="inline-start" /> Resume
              </Button>
            )}
            <Button variant="outline" onClick={() => void skip()} disabled={focus.phase === "idle"}>
              <SkipForward size={15} data-icon="inline-start" /> Skip
            </Button>
            <Button variant="ghost" onClick={() => void reset()}>
              <ArrowCounterClockwise size={15} data-icon="inline-start" /> Reset
            </Button>
          </div>
        </CardContent>
      </Card>

      <Section title="Durations">
        <Card>
          <CardContent className="divide-y">
            <SettingNumber
              label="Focus length"
              hint="minutes"
              value={settings.focusMinutes}
              onChange={(value) => updateSetting("focusMinutes", value)}
            />
            <SettingNumber
              label="Short break"
              hint="minutes"
              value={settings.shortBreakMinutes}
              onChange={(value) => updateSetting("shortBreakMinutes", value)}
            />
            <SettingNumber
              label="Long break"
              hint="minutes"
              value={settings.longBreakMinutes}
              onChange={(value) => updateSetting("longBreakMinutes", value)}
            />
            <SettingNumber
              label="Sessions before long break"
              hint="sessions"
              value={settings.sessionsBeforeLongBreak}
              onChange={(value) => updateSetting("sessionsBeforeLongBreak", value)}
            />
            <ToggleRow
              label="Auto-start breaks"
              hint="Begin the break as soon as focus ends"
              checked={settings.autoStartBreaks === "true"}
              onChange={(checked) => updateSetting("autoStartBreaks", String(checked))}
            />
            <ToggleRow
              label="Auto-start focus"
              hint="Begin the next focus session after a break"
              checked={settings.autoStartFocus === "true"}
              onChange={(checked) => updateSetting("autoStartFocus", String(checked))}
            />
          </CardContent>
        </Card>
      </Section>
    </Page>
  );
}

function SettingNumber({
  label,
  hint,
  value,
  onChange,
}: {
  label: string;
  hint: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-4 py-3">
      <div>
        <div className="text-sm">{label}</div>
        <div className="text-xs text-muted-foreground">{hint}</div>
      </div>
      <Input
        type="number"
        min={1}
        max={180}
        className="w-20"
        value={value}
        onChange={(event) => onChange(event.target.value)}
      />
    </div>
  );
}

function ToggleRow({
  label,
  hint,
  checked,
  onChange,
}: {
  label: string;
  hint: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-4 py-3">
      <div>
        <Label className="text-sm">{label}</Label>
        <div className="text-xs text-muted-foreground">{hint}</div>
      </div>
      <Switch checked={checked} onCheckedChange={onChange} />
    </div>
  );
}
