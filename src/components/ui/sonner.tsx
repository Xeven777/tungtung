import { Toaster as Sonner, type ToasterProps } from "sonner";

import { useStore } from "@/store";

/**
 * Sonner, wired to the app's own theme instead of `next-themes` (this project
 * has no Next.js). The CSS variables below are the ones shadcn/ui maps onto
 * Sonner so toasts inherit the surrounding design tokens.
 */
export function Toaster(props: ToasterProps) {
  const theme = useStore((s) => s.settings.theme);

  return (
    <Sonner
      theme={(theme as ToasterProps["theme"]) ?? "system"}
      position="bottom-right"
      richColors
      className="toaster group"
      style={
        {
          "--normal-bg": "var(--popover)",
          "--normal-text": "var(--popover-foreground)",
          "--normal-border": "var(--border)",
          "--border-radius": "var(--radius-lg)",
        } as React.CSSProperties
      }
      {...props}
    />
  );
}
