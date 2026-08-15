import { Skeleton, SkeletonCard } from "@/components/ui/skeleton";

// Route-level loading placeholder: a heading stub plus a few card skeletons,
// matching the cockpit layout so the shift on load stays minimal.
export function PageSkeleton({ cards = 3 }: { cards?: number }) {
  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <Skeleton className="h-3 w-24" />
        <Skeleton className="h-7 w-56" />
      </div>
      <div className="grid gap-4">
        {Array.from({ length: cards }).map((_, index) => (
          <SkeletonCard key={index} />
        ))}
      </div>
    </div>
  );
}
