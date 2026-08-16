import { Skeleton } from "@/components/ui/skeleton";

export default function MoreLoading() {
  return (
    <div className="space-y-6" aria-busy="true" aria-live="polite">
      <span className="sr-only">Carregando…</span>
      <div className="space-y-2">
        <Skeleton className="h-3 w-24" />
        <Skeleton className="h-7 w-32" />
      </div>
      <div className="space-y-2">
        {Array.from({ length: 6 }, (_, index) => (
          <Skeleton className="h-14 w-full" key={index} />
        ))}
      </div>
    </div>
  );
}
