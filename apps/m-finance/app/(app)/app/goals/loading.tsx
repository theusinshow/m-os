import { Skeleton, SkeletonCard } from "@/components/ui/skeleton";

export default function GoalsLoading() {
  return (
    <div className="space-y-6" aria-busy="true" aria-live="polite">
      <span className="sr-only">Carregando…</span>
      <div className="flex items-end justify-between gap-4">
        <div className="space-y-2">
          <Skeleton className="h-3 w-24" />
          <Skeleton className="h-7 w-56" />
        </div>
        <Skeleton className="h-11 w-32" />
      </div>
      <SkeletonCard className="min-h-28" />
      <div className="space-y-4">
        {Array.from({ length: 2 }, (_, index) => (
          <SkeletonCard className="min-h-44" key={index} />
        ))}
      </div>
    </div>
  );
}
