import { useEffect, useMemo, useRef, useState } from "react";
import {
  GearSix,
  MagnifyingGlass,
  Moon,
  Pause,
  Play,
  PlusCircle,
  Sparkle,
  Sun,
} from "@phosphor-icons/react";
import type { Icon } from "@phosphor-icons/react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { cn } from "@/lib/utils";
import { useStore } from "@/store";

interface Command {
  id: string;
  label: string;
  icon: Icon;
  run: () => void;
}

export function CommandPalette() {
  const open = useStore((s) => s.paletteOpen);
  const setOpen = useStore((s) => s.setPaletteOpen);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const commands = useMemo<Command[]>(
    () => [
      {
        id: "new-reminder",
        label: "Create reminder",
        icon: PlusCircle,
        run: () => useStore.getState().openQuickAdd(),
      },
      {
        id: "start-focus",
        label: "Start focus",
        icon: Play,
        run: () => void useStore.getState().focusStart(),
      },
      {
        id: "pause-focus",
        label: "Pause focus",
        icon: Pause,
        run: () => useStore.getState().focusPause(),
      },
      {
        id: "today",
        label: "Go to Today",
        icon: Sparkle,
        run: () => useStore.getState().setRoute("today"),
      },
      {
        id: "settings",
        label: "Open settings",
        icon: GearSix,
        run: () => useStore.getState().setRoute("settings"),
      },
      {
        id: "dark",
        label: "Toggle dark mode",
        icon: Moon,
        run: () => {
          const mode = useStore.getState().settings.theme === "dark" ? "light" : "dark";
          void useStore.getState().updateSetting("theme", mode);
        },
      },
      {
        id: "system",
        label: "Use system theme",
        icon: Sun,
        run: () => void useStore.getState().updateSetting("theme", "system"),
      },
    ],
    [],
  );

  const filtered = useMemo(
    () => commands.filter((command) => command.label.toLowerCase().includes(query.toLowerCase())),
    [commands, query],
  );

  useEffect(() => {
    if (!open) return;
    setQuery("");
    setActive(0);
    const timer = setTimeout(() => inputRef.current?.focus(), 20);
    return () => clearTimeout(timer);
  }, [open]);

  function run(command: Command | undefined) {
    if (!command) return;
    setOpen(false);
    command.run();
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent showCloseButton={false}
        className="gap-0 overflow-hidden p-0 duration-0 sm:max-w-lg data-open:animate-none">
        <DialogHeader className="sr-only">
          <DialogTitle>Command palette</DialogTitle>
        </DialogHeader>

        <div className="flex items-center gap-3 border-b px-5 py-4">
          <MagnifyingGlass size={16} className="text-muted-foreground" />
          <input
            ref={inputRef}
            value={query}
            placeholder="Search or run a command…"
            onChange={(event) => {
              setQuery(event.target.value);
              setActive(0);
            }}
            onKeyDown={(event) => {
              if (event.key === "ArrowDown") {
                event.preventDefault();
                setActive((current) => Math.min(current + 1, filtered.length - 1));
              }
              if (event.key === "ArrowUp") {
                event.preventDefault();
                setActive((current) => Math.max(current - 1, 0));
              }
              if (event.key === "Enter") {
                event.preventDefault();
                run(filtered[active]);
              }
            }}
            className="w-full bg-transparent text-sm outline-none placeholder:text-muted-foreground"
          />
        </div>

        <div className="max-h-80 overflow-y-auto py-1">
          {filtered.length === 0 ? (
            <div className="px-5 py-3 text-sm text-muted-foreground">No matching commands</div>
          ) : null}
          {filtered.map((command, index) => (
            <button
              key={command.id}
              type="button"
              onMouseEnter={() => setActive(index)}
              onClick={() => run(command)}
              className={cn(
                "flex w-full items-center gap-3 px-5 py-2.5 text-left text-sm",
                index === active && "bg-accent text-accent-foreground",
              )}
            >
              <command.icon size={16} />
              {command.label}
            </button>
          ))}
        </div>
      </DialogContent>
    </Dialog>
  );
}
