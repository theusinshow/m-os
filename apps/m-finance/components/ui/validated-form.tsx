"use client";

import { createContext, useActionState, useContext, useEffect, useRef } from "react";
import { initialFormState, type FormState } from "@/lib/form-state";
import { useToast } from "@/components/ui/toast";
import { cn } from "@/lib/utils";

type FormAction = (prevState: FormState, formData: FormData) => Promise<FormState>;

const FieldErrorContext = createContext<Record<string, string>>({});

function useFieldError(name: string) {
  return useContext(FieldErrorContext)[name];
}

export function ValidatedForm({
  action,
  children,
  className,
  successMessage,
  resetOnSuccess = false,
}: {
  action: FormAction;
  children: React.ReactNode;
  className?: string;
  successMessage: string;
  resetOnSuccess?: boolean;
}) {
  const [state, formAction] = useActionState(action, initialFormState);
  const { addToast } = useToast();
  const formRef = useRef<HTMLFormElement>(null);

  useEffect(() => {
    if (state.status === "success") {
      addToast(state.message ?? successMessage, "success");
      if (resetOnSuccess) {
        formRef.current?.reset();
      }
    } else if (state.status === "error" && state.message && !state.fieldErrors) {
      addToast(state.message, "error");
    }
  }, [state, addToast, successMessage, resetOnSuccess]);

  return (
    <FieldErrorContext.Provider value={state.fieldErrors ?? {}}>
      <form action={formAction} className={cn(className)} ref={formRef}>
        {children}
      </form>
    </FieldErrorContext.Provider>
  );
}

export function FieldError({ name }: { name: string }) {
  const error = useFieldError(name);
  if (!error) return null;
  return (
    <p className="mt-1.5 text-xs font-medium text-accent" role="alert">
      {error}
    </p>
  );
}

const ERROR_BORDER = "border-accent ring-1 ring-accent/30";

export function ValidatedInput({
  name,
  className,
  ...props
}: React.InputHTMLAttributes<HTMLInputElement> & { name: string }) {
  const error = useFieldError(name);
  return (
    <>
      <input
        aria-invalid={error ? true : undefined}
        className={cn(className, error && ERROR_BORDER)}
        name={name}
        {...props}
      />
      <FieldError name={name} />
    </>
  );
}

export function ValidatedSelect({
  name,
  className,
  children,
  ...props
}: React.SelectHTMLAttributes<HTMLSelectElement> & { name: string }) {
  const error = useFieldError(name);
  return (
    <>
      <select
        aria-invalid={error ? true : undefined}
        className={cn(className, error && ERROR_BORDER)}
        name={name}
        {...props}
      >
        {children}
      </select>
      <FieldError name={name} />
    </>
  );
}
