import Link from "next/link";
import { CircleDashed } from "lucide-react";
import { DashboardCard } from "@/components/dashboard/dashboard-card";

export function EmptyState({
  actionHref,
  actionLabel,
  description,
  title,
}: {
  actionHref?: string;
  actionLabel?: string;
  description: string;
  title: string;
}) {
  return (
    <DashboardCard>
      <div className="flex min-h-48 flex-col justify-center">
        <div className="mb-4 flex h-11 w-11 items-center justify-center rounded-md border border-border-subtle bg-background-elevated text-text-muted">
          <CircleDashed size={20} />
        </div>
        <h2 className="text-lg font-semibold text-text-primary">{title}</h2>
        <p className="mt-2 max-w-xl text-sm leading-6 text-text-muted">{description}</p>
        {actionHref && actionLabel ? (
          <Link
            className="focus-ring mt-5 inline-flex min-h-11 w-fit items-center rounded-md border border-border-subtle px-4 text-sm font-semibold text-text-secondary transition duration-200 hover:border-border-default hover:bg-background-hover hover:text-text-primary"
            href={actionHref}
          >
            {actionLabel}
          </Link>
        ) : null}
      </div>
    </DashboardCard>
  );
}
