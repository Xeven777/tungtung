export interface RecurrenceOption {
  value: string;
  label: string;
}

export const RECURRENCE_OPTIONS: RecurrenceOption[] = [
  { value: "", label: "Does not repeat" },
  { value: "FREQ=DAILY", label: "Every day" },
  { value: "FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR", label: "Every weekday" },
  { value: "FREQ=WEEKLY", label: "Every week" },
  { value: "FREQ=MONTHLY", label: "Every month" },
];

export function buildRule(
  freq: "HOURLY" | "DAILY" | "WEEKLY" | "MONTHLY",
  opts: { interval?: number; byDay?: string[] } = {},
): string {
  const parts = [`FREQ=${freq}`];
  if (opts.interval && opts.interval > 1) parts.push(`INTERVAL=${opts.interval}`);
  if (opts.byDay?.length) parts.push(`BYDAY=${opts.byDay.join(",")}`);
  return parts.join(";");
}

const DAY_LABELS: Record<string, string> = {
  MO: "Mon",
  TU: "Tue",
  WE: "Wed",
  TH: "Thu",
  FR: "Fri",
  SA: "Sat",
  SU: "Sun",
};

interface ParsedRule {
  freq: string;
  interval: number;
  byDay: string[];
}

export function parseRule(rule: string): ParsedRule | null {
  const parsed: ParsedRule = { freq: "", interval: 1, byDay: [] };
  for (const chunk of rule.split(";")) {
    const [rawKey, rawValue] = chunk.split("=");
    if (!rawKey || !rawValue) continue;
    const key = rawKey.trim().toUpperCase();
    const value = rawValue.trim().toUpperCase();
    if (key === "FREQ") parsed.freq = value;
    if (key === "INTERVAL") parsed.interval = Number(value) || 1;
    if (key === "BYDAY") parsed.byDay = value.split(",").filter(Boolean);
  }
  return parsed.freq ? parsed : null;
}

export function describeRule(rule: string | null): string | null {
  if (!rule) return null;
  const parsed = parseRule(rule);
  if (!parsed) return "Repeats";

  const every = parsed.interval > 1 ? `Every ${parsed.interval} ` : "Every ";

  switch (parsed.freq) {
    case "HOURLY":
      return parsed.interval > 1 ? `Every ${parsed.interval} hours` : "Every hour";
    case "DAILY":
      return parsed.interval > 1 ? `Every ${parsed.interval} days` : "Every day";
    case "WEEKLY": {
      if (parsed.byDay.length === 5 && !parsed.byDay.includes("SA") && !parsed.byDay.includes("SU")) {
        return "Every weekday";
      }
      if (parsed.byDay.length) {
        return `${every}week on ${parsed.byDay.map((d) => DAY_LABELS[d] ?? d).join(", ")}`;
      }
      return parsed.interval > 1 ? `Every ${parsed.interval} weeks` : "Every week";
    }
    case "MONTHLY":
      return parsed.interval > 1 ? `Every ${parsed.interval} months` : "Every month";
    default:
      return "Repeats";
  }
}
