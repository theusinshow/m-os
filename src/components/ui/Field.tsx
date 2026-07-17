import type {
  InputHTMLAttributes,
  ReactNode,
  SelectHTMLAttributes,
  TextareaHTMLAttributes,
} from "react";
import { cn } from "@/lib/cn";

// `border-strong` (nao `border`): a borda e a unica coisa que identifica o
// controle. A borda discreta dos cards e decorativa; a de um campo e funcional,
// e por isso segue o minimo de 3:1 do WCAG 1.4.11.
const baseControl =
  "w-full rounded border border-border-strong bg-bg px-3 py-2 text-sm text-text " +
  "placeholder:text-text-subtle focus:border-accent focus:outline-none " +
  "focus-visible:outline-none disabled:opacity-50";

interface FieldProps {
  label: string;
  htmlFor?: string;
  hint?: string;
  required?: boolean;
  children: ReactNode;
}

/** Rotulo + controle + dica, com espacamento consistente. */
export function Field({ label, htmlFor, hint, required, children }: FieldProps) {
  return (
    <div className="space-y-1.5">
      <label
        htmlFor={htmlFor}
        className="block text-xs font-medium text-text-muted"
      >
        {label}
        {required && <span className="ml-0.5 text-danger">*</span>}
      </label>
      {children}
      {hint && <p className="text-2xs text-text-subtle">{hint}</p>}
    </div>
  );
}

export function Input({
  className,
  ...props
}: InputHTMLAttributes<HTMLInputElement>) {
  return <input className={cn(baseControl, className)} {...props} />;
}

export function Textarea({
  className,
  ...props
}: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return (
    <textarea className={cn(baseControl, "resize-y", className)} {...props} />
  );
}

export function Select({
  className,
  children,
  ...props
}: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select className={cn(baseControl, className)} {...props}>
      {children}
    </select>
  );
}
