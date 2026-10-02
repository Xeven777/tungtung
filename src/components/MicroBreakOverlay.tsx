import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { formatClock } from "@/lib/dates";
import { cn } from "@/lib/utils";
import { useStore } from "@/store";

/** The presets named in the spec, in ascending order of commitment. */
const PRESETS = [
  { label: "Eye break", seconds: 30 },
  { label: "Stretch", seconds: 120 },
  { label: "Short walk", seconds: 300 },
  { label: "Long break", seconds: 600 },
];

/**
 * A break should cover the whole screen, so the window goes fullscreen for the
 * duration and is restored afterwards. If the window is hidden in the tray
 * there is nothing to cover — the OS notification is the nudge instead.
 */
export function MicroBreakOverlay() {
  const micro = useStore((s) => s.micro);
  const start = useStore((s) => s.startMicroBreak);
  const dismiss = useStore((s) => s.dismissMicroBreak);

  useEffect(() => {
    if (!micro.open) return;

    const win = getCurrentWindow();
    const previous = { wasFullscreen: false };
    const ready = (async () => {
      const visible = await win.isVisible().catch(() => true);
      if (!visible) return;
      previous.wasFullscreen = await win.isFullscreen().catch(() => false);
      await win.setFullscreen(true).catch(() => undefined);
    })();

    return () => {
      void ready.then(() => {
        if (!previous.wasFullscreen) void win.setFullscreen(false).catch(() => undefined);
      });
    };
  }, [micro.open]);

  useEffect(() => {
    if (!micro.open) return;
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") dismiss();
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [micro.open, dismiss]);

  if (!micro.open) return null;

  const progress = micro.planned
    ? ((micro.planned - micro.remaining) / micro.planned) * 100
    : 0;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Micro break"
      className="fixed inset-0 z-50 flex items-center justify-center bg-background/92 backdrop-blur-sm animate-in fade-in duration-150"
    >
      <div className="w-[420px] max-w-[calc(100vw-3rem)] rounded-2xl border bg-card p-8 shadow-xl">
        <div className="text-xs font-medium tracking-wide text-muted-foreground uppercase">
          Micro break
        </div>

        <div className="mt-5 text-6xl leading-none font-medium tabular-nums tracking-tight">
          {formatClock(micro.remaining)}
        </div>

        <p className="mt-3 text-sm text-muted-foreground">
          Look away from the screen and let your eyes rest.
        </p>

        <Progress value={progress} className="mt-6" />

        <div className="mt-6 flex flex-wrap gap-2">
          {PRESETS.map((preset) => (
            <Button
              key={preset.label}
              size="sm"
              variant={micro.planned === preset.seconds ? "secondary" : "outline"}
              className={cn(
                micro.planned !== preset.seconds && "text-muted-foreground hover:text-foreground",
              )}
              onClick={() => start(preset.seconds)}
            >
              {preset.label}
            </Button>
          ))}
        </div>

        <div className="mt-8 flex justify-end">
          <Button onClick={dismiss}>Done</Button>
        </div>
      </div>
    </div>
  );
}
