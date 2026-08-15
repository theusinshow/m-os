import { cn } from "@/lib/utils";

type ButtonVariant = "primary" | "secondary" | "ghost";

const base =
  "focus-ring inline-flex min-h-11 items-center justify-center gap-2 rounded-md px-4 text-sm font-semibold transition duration-200 disabled:cursor-wait disabled:opacity-75";

const variants: Record<ButtonVariant, string> = {
  primary:
    "bg-accent-strong text-text-primary shadow-lg shadow-accent/25 hover:bg-accent-strong-hover active:scale-[0.985]",
  secondary:
    "border border-border-default bg-background-elevated text-text-secondary hover:border-border-strong hover:bg-background-hover hover:text-text-primary active:scale-[0.985]",
  ghost:
    "border border-border-subtle text-text-secondary hover:bg-background-hover hover:text-text-primary",
};

/**
 * Shared action button for non-form clicks (the form-submit equivalent lives in
 * FormSubmitButton). Keeps primary/secondary styling consistent across screens.
 */
export function Button({
  variant = "primary",
  className,
  ...props
}: React.ButtonHTMLAttributes<HTMLButtonElement> & { variant?: ButtonVariant }) {
  return <button className={cn(base, variants[variant], className)} {...props} />;
}
