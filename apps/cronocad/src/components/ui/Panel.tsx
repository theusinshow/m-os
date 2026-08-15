import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/cn";

interface PanelProps extends HTMLAttributes<HTMLDivElement> {
  children: ReactNode;
}

/** Superficie discreta com borda sutil (direcao visual da secao 17). */
export function Panel({ children, className, ...props }: PanelProps) {
  return (
    <div
      className={cn(
        "rounded-lg border border-border bg-surface",
        className,
      )}
      {...props}
    >
      {children}
    </div>
  );
}

interface PanelHeaderProps {
  title: string;
  action?: ReactNode;
}

export function PanelHeader({ title, action }: PanelHeaderProps) {
  return (
    <div className="flex items-center justify-between border-b border-border px-4 py-3">
      <h2 className="text-xs font-semibold uppercase tracking-wide text-text-muted">
        {title}
      </h2>
      {action}
    </div>
  );
}
