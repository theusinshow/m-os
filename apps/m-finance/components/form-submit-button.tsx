"use client";

import { useFormStatus } from "react-dom";
import { LoaderCircle } from "lucide-react";
import { TriangleMark } from "@/components/brand/triangle-mark";
import { cn } from "@/lib/utils";

export function FormSubmitButton({
  children,
  pendingLabel = "Salvando...",
  variant = "primary",
}: {
  children: React.ReactNode;
  pendingLabel?: string;
  variant?: "primary" | "secondary" | "danger" | "success";
}) {
  const { pending } = useFormStatus();

  return (
    <button
      className={cn(
        "clip-notch sheen group focus-ring relative inline-flex min-h-11 items-center justify-center gap-2 px-4 text-sm font-semibold tracking-tight transition duration-200 disabled:cursor-wait disabled:opacity-75",
        variant === "primary" &&
          "bg-accent-strong text-text-primary shadow-lg shadow-accent/25 hover:bg-accent-strong-hover active:scale-[0.985]",
        variant === "secondary" &&
          "border border-border-default bg-background-elevated text-text-secondary hover:border-border-strong hover:bg-background-hover hover:text-text-primary active:scale-[0.985]",
        variant === "danger" &&
          "border border-accent-border text-accent hover:bg-accent-soft active:scale-[0.985]",
        variant === "success" &&
          "bg-status-positive text-text-inverse shadow-lg shadow-status-positive/20 hover:brightness-110 active:scale-[0.985]",
      )}
      disabled={pending}
      aria-busy={pending}
      type="submit"
    >
      {pending ? (
        <>
          <LoaderCircle className="animate-spin" size={16} aria-hidden="true" />
          <span>{pendingLabel}</span>
        </>
      ) : (
        <>
          <TriangleMark
            className={cn(
              "rotate-90 transition-transform duration-300 group-hover:translate-x-0.5",
              variant === "primary" ? "opacity-90" : "opacity-60",
            )}
            size={11}
            variant="solid"
          />
          <span>{children}</span>
        </>
      )}
    </button>
  );
}
