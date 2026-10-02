import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { CornersIn, CornersOut, Minus, X } from "@phosphor-icons/react";
import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

/**
 * Mirrors the `ResizeDirection` union in `@tauri-apps/api/window`, which declares
 * the type internally without re-exporting it.
 */
type ResizeDirection =
  | "East"
  | "North"
  | "NorthEast"
  | "NorthWest"
  | "South"
  | "SouthEast"
  | "SouthWest"
  | "West";

/**
 * The window is frameless (`decorations: false`) so XFCE's system buttons never
 * appear next to the app's own UI. Two things the window manager used to do have
 * to be replaced by hand:
 *
 * - Dragging: handled by `data-tauri-drag-region`, which Tauri wires directly.
 *   It only applies to the element it sits on, so the bar and the filler both
 *   carry it. Double-click-to-maximise comes free with this.
 * - Resizing: a frameless GTK window has no border to grab, so `resizable: true`
 *   alone does nothing — the invisible grips below call `startResizeDragging`.
 */

const GRIPS: { direction: ResizeDirection; className: string }[] = [
  { direction: "North", className: "inset-x-2.5 top-0 h-1 cursor-ns-resize" },
  {
    direction: "South",
    className: "inset-x-2.5 bottom-0 h-1 cursor-ns-resize",
  },
  { direction: "West", className: "inset-y-2.5 left-0 w-1 cursor-ew-resize" },
  { direction: "East", className: "inset-y-2.5 right-0 w-1 cursor-ew-resize" },
  {
    direction: "NorthWest",
    className: "top-0 left-0 size-2.5 cursor-nwse-resize",
  },
  {
    direction: "NorthEast",
    className: "top-0 right-0 size-2.5 cursor-nesw-resize",
  },
  {
    direction: "SouthWest",
    className: "bottom-0 left-0 size-2.5 cursor-nesw-resize",
  },
  {
    direction: "SouthEast",
    className: "right-0 bottom-0 size-2.5 cursor-nwse-resize",
  },
];

export function TitleBar() {
  const [maximized, setMaximized] = useState(false);

  // Drives the rounded corners in `index.css`; a maximised window fills the
  // screen, so rounding it would only clip the desktop at the edges.
  useEffect(() => {
    document.documentElement.classList.toggle("maximized", maximized);
  }, [maximized]);

  useEffect(() => {
    const win = getCurrentWindow();
    let disposed = false;
    let unlisten: (() => void) | undefined;

    const sync = () => {
      void win.isMaximized().then((value) => {
        if (!disposed) setMaximized(value);
      });
    };
    sync();

    // `resized` is the only signal the window manager gives for maximising,
    // restoring, and for a double-click on the drag region. It fires on every
    // frame of a resize drag, so debounce to the trailing edge rather than
    // round-tripping to the backend hundreds of times.
    let settle: ReturnType<typeof setTimeout> | undefined;
    void win
      .onResized(() => {
        clearTimeout(settle);
        settle = setTimeout(sync, 150);
      })
      .then((un) => {
        if (disposed) un();
        else unlisten = un;
      });

    return () => {
      disposed = true;
      clearTimeout(settle);
      unlisten?.();
    };
  }, []);

  return (
    <>
      <header
        data-tauri-drag-region
        className="flex h-9 shrink-0 items-center justify-end border-b bg-sidebar select-none px-4 gap-2.5"
      >
        {/* The empty drag area. `data-tauri-drag-region` does not inherit. */}
        <div data-tauri-drag-region className="flex-1" />
        <WindowControl
          classname="bg-yellow-500 hover:text-white text-yellow-500"
          label="Minimise"
          onClick={() => getCurrentWindow().minimize()}
        >
          <Minus size={14} weight="bold" />
        </WindowControl>
        <WindowControl
          classname="bg-green-500 hover:text-white text-green-500"
          label={maximized ? "Restore" : "Maximise"}
          onClick={() => getCurrentWindow().toggleMaximize()}
        >
          {maximized ? (
            <CornersIn size={14} weight="bold" />
          ) : (
            <CornersOut size={14} weight="bold" />
          )}
        </WindowControl>
        <WindowControl
          label="Close"
          classname="bg-red-500 hover:text-white text-red-500"
          onClick={() => getCurrentWindow().close()}
        >
          <X size={14} weight="bold" />
        </WindowControl>
      </header>

      {/* A maximised window has nothing to resize. */}
      {maximized
        ? null
        : GRIPS.map((grip) => (
            <div
              key={grip.direction}
              aria-hidden
              className={cn("fixed z-40", grip.className)}
              onPointerDown={(event) => {
                if (event.button !== 0) return;
                event.preventDefault();
                void getCurrentWindow().startResizeDragging(grip.direction);
              }}
            />
          ))}
    </>
  );
}

function WindowControl({
  label,
  classname,
  onClick,
  children,
}: {
  label: string;
  classname?: string;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className={cn(
        "inline-flex size-4.5 items-center justify-center outline-none transition-colors duration-150 ease-out focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:ring-inset rounded-full",
        classname,
      )}
    >
      {children}
    </button>
  );
}
