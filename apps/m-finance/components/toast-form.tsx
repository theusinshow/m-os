"use client";

import { useToast } from "@/components/ui/toast";
import { cn } from "@/lib/utils";

export function ToastForm({
  action,
  children,
  className,
  successMessage,
  errorMessage = "Não foi possível concluir a ação. Tente novamente.",
}: {
  action: (formData: FormData) => Promise<void>;
  children: React.ReactNode;
  className?: string;
  successMessage: string;
  errorMessage?: string;
}) {
  const { addToast } = useToast();

  // An async function passed to `form action` keeps useFormStatus().pending
  // true while awaiting, so FormSubmitButton still shows its loading state.
  async function handleAction(formData: FormData) {
    try {
      await action(formData);
      addToast(successMessage, "success");
    } catch (error) {
      // Next.js redirects/notFound throw control-flow errors we must rethrow.
      if (error instanceof Error && error.message === "NEXT_REDIRECT") {
        throw error;
      }
      const digest = (error as { digest?: string })?.digest;
      if (typeof digest === "string" && digest.startsWith("NEXT_REDIRECT")) {
        throw error;
      }
      addToast(errorMessage, "error");
    }
  }

  return (
    <form action={handleAction} className={cn(className)}>
      {children}
    </form>
  );
}
