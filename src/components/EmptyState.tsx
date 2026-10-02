import type { ReactNode } from "react";

interface Props {
  title: string;
  body?: string;
  icon?: ReactNode;
  action?: ReactNode;
}

export function EmptyState({ title, body, icon, action }: Props) {
  return (
    <div className="flex flex-col items-center gap-3 rounded-xl border border-dashed px-6 py-12 text-center text-sm text-muted-foreground">
      {icon}
      <strong className="text-base font-medium text-foreground">{title}</strong>
      {body ? <span>{body}</span> : null}
      {action ? <div className="mt-1">{action}</div> : null}
    </div>
  );
}
