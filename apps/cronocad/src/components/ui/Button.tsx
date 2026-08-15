import type { ButtonHTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/cn";

type Variant = "primary" | "secondary" | "ghost" | "danger";
type Size = "sm" | "md";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
  icon?: ReactNode;
  children?: ReactNode;
}

const VARIANT: Record<Variant, string> = {
  primary:
    "bg-accent text-accent-contrast hover:bg-accent-hover border border-transparent",
  secondary:
    "bg-surface-raised text-text hover:bg-surface-hover border border-border",
  ghost:
    "bg-transparent text-text-muted hover:text-text hover:bg-surface-hover border border-transparent",
  danger:
    "bg-transparent text-danger hover:bg-danger-muted border border-transparent",
};

const SIZE: Record<Size, string> = {
  sm: "h-8 px-3 text-xs gap-1.5",
  md: "h-9 px-4 text-sm gap-2",
};

export function Button({
  variant = "secondary",
  size = "md",
  icon,
  children,
  className,
  ...props
}: ButtonProps) {
  return (
    <button
      className={cn(
        "inline-flex items-center justify-center rounded font-medium",
        "transition-colors duration-fast",
        "disabled:pointer-events-none disabled:opacity-50",
        VARIANT[variant],
        SIZE[size],
        className,
      )}
      {...props}
    >
      {icon}
      {children}
    </button>
  );
}
