import {
  Bell,
  Clock,
  ClockCounterClockwise,
  Coffee,
  Fire,
  GearSix,
  Sparkle,
} from "@phosphor-icons/react";
import type { Icon } from "@phosphor-icons/react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { formatClock } from "@/lib/dates";
import { useStore, type Route } from "@/store";

const NAV: { route: Route; label: string; icon: Icon }[] = [
  { route: "today", label: "Today", icon: Sparkle },
  { route: "reminders", label: "Reminders", icon: Bell },
  { route: "focus", label: "Focus", icon: Clock },
  { route: "breaks", label: "Breaks", icon: Coffee },
  { route: "habits", label: "Habits", icon: Fire },
  { route: "history", label: "History", icon: ClockCounterClockwise },
];

function NavButton({
  icon: Icon_,
  label,
  active,
  onClick,
}: {
  icon: Icon;
  label: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <Button
      variant="ghost"
      aria-current={active ? "page" : undefined}
      onClick={onClick}
      className={cn(
        "h-9 w-full justify-start gap-3 px-2.5 text-sm font-normal text-muted-foreground",
        "max-[860px]:justify-center max-[860px]:px-0",
        active && "bg-primary/10 font-medium text-foreground hover:bg-primary/15",
      )}
    >
      <Icon_ size={17} />
      <span className="max-[860px]:hidden">{label}</span>
    </Button>
  );
}

export function Sidebar() {
  const route = useStore((s) => s.route);
  const setRoute = useStore((s) => s.setRoute);
  const focus = useStore((s) => s.focus);

  const running = focus.phase !== "idle" && focus.running;

  return (
    <aside className="flex h-full flex-col gap-3 border-r bg-sidebar px-5 py-6 max-[860px]:items-center max-[860px]:px-3">
      <div className="flex items-center gap-2 px-2.5 pb-5 max-[860px]:px-0">
        <img
          src="../../src-tauri/icons/Square71x71Logo.png"
          alt="TungTung"
          width={40}
          height={40}
          className="size-10 object-cover"
        />
        <span className="max-[860px]:hidden text-xl font-bold cursor-pointer">TungTung</span>
      </div>

      <nav className="flex flex-col gap-0.5">
        {NAV.map((item) => (
          <NavButton
            key={item.route}
            icon={item.icon}
            label={item.label}
            active={route === item.route}
            onClick={() => setRoute(item.route)}
          />
        ))}
      </nav>

      <div className="flex-1" />

      <div className="flex flex-col gap-3 border-t pt-5 max-[860px]:w-full">
        {running ? (
          <div className="flex items-center gap-2 px-2.5 text-xs text-muted-foreground max-[860px]:justify-center max-[860px]:px-0">
            <span className="size-2 shrink-0 rounded-full bg-emerald-500" />
            <span className="max-[860px]:hidden tabular-nums">
              {formatClock(focus.remaining)}
            </span>
          </div>
        ) : null}
        <NavButton
          icon={GearSix}
          label="Settings"
          active={route === "settings"}
          onClick={() => setRoute("settings")}
        />
      </div>
    </aside>
  );
}
