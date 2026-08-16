import { Skeleton } from "@/components/ui/skeleton";

export default function CalendarLoading() {
  return (
    <div className="space-y-6" aria-busy="true" aria-live="polite">
      <span className="sr-only">Carregando…</span>
      <div className="space-y-2">
        <Skeleton className="h-3 w-24" />
        <Skeleton className="h-7 w-56" />
      </div>
      <section className="grid gap-4 xl:grid-cols-[1fr_380px]">
        <div className="rounded-xl border border-border-subtle bg-background-card/95 p-5 shadow-xl shadow-black/15">
          <div className="grid grid-cols-7 gap-1.5 sm:gap-2">
            {Array.from({ length: 7 }, (_, index) => (
              <Skeleton className="h-4 w-full" key={`weekday-${index}`} />
            ))}
            {Array.from({ length: 35 }, (_, index) => (
              <Skeleton className="min-h-14 w-full sm:min-h-28" key={`day-${index}`} />
            ))}
          </div>
        </div>
        <div className="rounded-xl border border-border-subtle bg-background-card/95 p-5 shadow-xl shadow-black/15">
          <Skeleton className="h-4 w-1/3" />
          <div className="mt-4 space-y-3">
            {Array.from({ length: 3 }, (_, index) => (
              <Skeleton className="h-24 w-full" key={index} />
            ))}
          </div>
        </div>
      </section>
    </div>
  );
}
