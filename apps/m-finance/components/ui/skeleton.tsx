import { cn } from "@/lib/utils";

export function Skeleton({ className }: { className?: string }) {
  return (
    <div
      aria-hidden="true"
      className={cn(
        "animate-pulse rounded-md border border-border-subtle bg-background-elevated/80",
        className,
      )}
    />
  );
}

export function SkeletonCard({ className }: { className?: string }) {
  return (
    <div
      aria-hidden="true"
      className={cn(
        "rounded-xl border border-border-subtle bg-background-card/95 p-5 shadow-xl shadow-black/15",
        className,
      )}
    >
      <Skeleton className="h-4 w-1/3" />
      <Skeleton className="mt-4 h-8 w-2/3" />
      <Skeleton className="mt-3 h-4 w-full" />
      <Skeleton className="mt-2 h-4 w-4/5" />
    </div>
  );
}
