import { Skeleton, SkeletonCard } from "@/components/ui/skeleton";

export default function SettingsLoading() {
  return (
    <div className="space-y-6" aria-busy="true" aria-live="polite">
      <span className="sr-only">Carregando…</span>
      <div className="space-y-2">
        <Skeleton className="h-3 w-24" />
        <Skeleton className="h-7 w-48" />
      </div>
      <section className="grid gap-4 lg:grid-cols-2">
        <SkeletonCard className="min-h-40" />
        <SkeletonCard className="min-h-40" />
      </section>
      <SkeletonCard className="min-h-56" />
      <SkeletonCard className="min-h-32" />
    </div>
  );
}
