import { Skeleton, SkeletonCard } from "@/components/ui/skeleton";

export default function AppLoading() {
  return (
    <div className="space-y-6" aria-busy="true" aria-live="polite">
      <span className="sr-only">Carregando…</span>
      <div className="space-y-3">
        <Skeleton className="h-3 w-32" />
        <Skeleton className="h-8 w-64" />
      </div>
      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        {Array.from({ length: 4 }, (_, index) => (
          <SkeletonCard key={index} />
        ))}
      </div>
      <div className="grid gap-4 lg:grid-cols-[1.4fr_1fr]">
        <SkeletonCard className="min-h-64" />
        <SkeletonCard className="min-h-64" />
      </div>
    </div>
  );
}
