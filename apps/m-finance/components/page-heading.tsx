import { TriangleMark } from "@/components/brand/triangle-mark";

export function PageHeading({
  eyebrow,
  title,
  children,
}: {
  eyebrow: string;
  title: string;
  children?: React.ReactNode;
}) {
  return (
    <div className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
      <div className="min-w-0">
        <p className="flex items-center gap-2 text-sm font-medium uppercase tracking-[0.18em] text-text-muted">
          <TriangleMark className="shrink-0 text-accent" size={11} variant="solid" />
          {eyebrow}
        </p>
        <h1 className="mt-2 text-balance font-display text-2xl font-semibold tracking-tight text-text-primary sm:text-3xl">
          {title}
        </h1>
      </div>
      {children ? <div className="w-full md:w-auto">{children}</div> : null}
    </div>
  );
}
