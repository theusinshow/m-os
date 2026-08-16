import { Skeleton, SkeletonCard } from "@/components/ui/skeleton";

export default function SimulatorLoading() {
  return (
    <div className="space-y-6" aria-busy="true" aria-live="polite">
      <span className="sr-only">Carregando…</span>
      <div className="space-y-2">
        <Skeleton className="h-3 w-24" />
        <Skeleton className="h-7 w-72" />
      </div>
      <SkeletonCard className="min-h-28" />
      <SkeletonCard className="min-h-64" />
      <SkeletonCard className="min-h-40" />
    </div>
  );
}
