"use client";

import { useFormStatus } from "react-dom";
import { LoaderCircle, LogOut } from "lucide-react";

export function SignOutButton() {
  const { pending } = useFormStatus();

  return (
    <button
      aria-busy={pending}
      aria-label={pending ? "Saindo" : "Sair"}
      className="focus-ring flex h-11 w-11 items-center justify-center rounded-md border border-border-subtle bg-background-secondary text-text-muted transition duration-200 hover:border-border-default hover:bg-background-hover hover:text-text-primary disabled:cursor-wait disabled:opacity-75"
      disabled={pending}
      type="submit"
    >
      {pending ? (
        <LoaderCircle className="animate-spin" size={16} aria-hidden="true" />
      ) : (
        <LogOut size={16} aria-hidden="true" />
      )}
    </button>
  );
}
