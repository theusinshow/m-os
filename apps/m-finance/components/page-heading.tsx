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
      <div>
        <p className="flex items-center gap-2 text-sm font-medium uppercase tracking-[0.18em] text-text-muted">
          <TriangleMark className="text-accent" size={11} variant="solid" />
          {eyebrow}
        </p>
        <h1 className="mt-2 font-display text-3xl font-semibold tracking-tight text-text-primary">
          {title}
        </h1>
      </div>
      {children}
    </div>
  );
}
