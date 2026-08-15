import { TriangleMark } from "@/components/brand/triangle-mark";
import { cn } from "@/lib/utils";

export function DashboardCard({
  children,
  className,
  title,
  description,
  accent = false,
}: {
  children: React.ReactNode;
  className?: string;
  title?: string;
  description?: string;
  accent?: boolean;
}) {
  return (
    <section
      className={cn(
        "relative rounded-xl border border-border-subtle bg-background-card/95 p-5 shadow-xl shadow-black/15 ring-1 ring-white/[0.02] transition duration-300 hover:border-border-default hover:shadow-2xl hover:shadow-black/25",
        accent && "clip-notch-lg",
        className,
      )}
    >
      {accent ? (
        <span
          aria-hidden="true"
          className="absolute right-0 top-0 h-[1.1rem] w-[1.1rem] bg-accent/60"
          style={{ clipPath: "polygon(100% 0, 0 0, 100% 100%)" }}
        />
      ) : null}
      {title ? (
        <div className="mb-4">
          <h2 className="flex items-center gap-2 text-sm font-semibold uppercase tracking-[0.16em] text-text-muted">
            <TriangleMark className="text-accent/70" size={10} variant="solid" />
            {title}
          </h2>
          {description ? (
            <p className="mt-1.5 text-sm leading-5 text-text-muted">{description}</p>
          ) : null}
        </div>
      ) : null}
      {children}
    </section>
  );
}
