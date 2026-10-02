import type { ReactNode } from "react";

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { previewSound } from "@/lib/sounds";

/**
 * The two row shapes shared by every settings-style screen. `Row` is a label +
 * hint on the left and a single control on the right; `SoundRow` is a sound
 * picker that previews whatever you pick.
 */
export function Row({
  label,
  hint,
  control,
}: {
  label: ReactNode;
  hint?: ReactNode;
  control: ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-4 py-3">
      <div className="min-w-0">
        <div className="text-sm">{label}</div>
        {hint ? <div className="text-xs text-muted-foreground">{hint}</div> : null}
      </div>
      {control}
    </div>
  );
}

export function SoundRow({
  label,
  value,
  sounds,
  volume,
  onChange,
}: {
  label: string;
  value: string;
  sounds: { id: string; name: string; filePath?: string | null }[];
  volume: number;
  onChange: (value: string) => void;
}) {
  const byId = (id: string) => sounds.find((s) => s.id === id);
  return (
    <Row
      label={label}
      control={
        <div className="flex items-center gap-2">
          <Select
            value={value}
            onValueChange={(next) => {
              onChange(next);
              const sound = byId(next);
              previewSound(next, volume, sound?.filePath ?? null);
            }}
          >
            <SelectTrigger className="w-40">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {sounds.map((sound) => (
                <SelectItem key={sound.id} value={sound.id}>
                  {sound.name}
                </SelectItem>
              ))}
              <SelectItem value="none">None</SelectItem>
            </SelectContent>
          </Select>
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              const sound = byId(value);
              previewSound(value, volume, sound?.filePath ?? null);
            }}
          >
            Test
          </Button>
        </div>
      }
    />
  );
}
