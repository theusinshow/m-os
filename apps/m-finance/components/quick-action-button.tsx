import Link from "next/link";
import { ArrowUpRight } from "lucide-react";
import { TriangleMark } from "@/components/brand/triangle-mark";
import { cn } from "@/lib/utils";

export function QuickActionButton({
  href,
  label,
  variant = "primary",
}: {
  href: string;
  label: string;
  variant?: "primary" | "secondary";
}) {
  return (
    <Link
      className={cn(
        "clip-notch sheen group focus-ring flex min-h-11 items-center justify-between gap-3 px-3.5 text-sm font-semibold tracking-tight transition duration-200 active:scale-[0.985]",
        variant === "primary"
          ? "bg-accent-strong text-text-primary shadow-lg shadow-accent/25 hover:bg-accent-strong-hover"
          : "border border-border-default bg-background-elevated text-text-secondary hover:border-border-strong hover:bg-background-hover hover:text-text-primary",
      )}
      href={href}
    >
      <span className="flex items-center gap-2.5">
        <TriangleMark
          className={cn(
            "transition-transform duration-300 group-hover:-translate-y-0.5",
            variant === "primary" ? "opacity-90" : "text-accent opacity-70",
          )}
          size={11}
          variant="solid"
        />
        {label}
      </span>
      <ArrowUpRight
        className="transition-transform duration-300 group-hover:translate-x-0.5 group-hover:-translate-y-0.5"
        size={16}
      />
    </Link>
  );
}
