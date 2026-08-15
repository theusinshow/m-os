import { CircleDashed } from "lucide-react";
import { cn } from "@/lib/utils";

/**
 * Lightweight empty state for use inside cards/lists, where the full-card
 * <EmptyState> would be too heavy. Standardizes the scattered "nothing here
 * yet" notes with a consistent surface and a subtle icon.
 */
export function InlineEmpty({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex items-center gap-2.5 rounded-lg border border-border-subtle bg-background-elevated p-4 text-sm text-text-muted",
        className,
      )}
    >
      <CircleDashed aria-hidden="true" className="shrink-0 text-text-muted/70" size={16} />
      <span>{children}</span>
    </div>
  );
}
