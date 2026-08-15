"use client";

import { useState } from "react";
import { useFormStatus } from "react-dom";
import { LoaderCircle, Trash2 } from "lucide-react";

function ConfirmSubmit() {
  const { pending } = useFormStatus();

  return (
    <button
      aria-busy={pending}
      className="clip-notch focus-ring inline-flex min-h-10 items-center gap-2 bg-accent-strong px-3 text-xs font-semibold text-text-primary transition duration-200 hover:bg-accent-strong-hover active:scale-[0.985] disabled:cursor-wait disabled:opacity-70"
      disabled={pending}
      type="submit"
    >
      {pending ? (
        <>
          <LoaderCircle className="animate-spin" size={14} aria-hidden="true" />
          Excluindo...
        </>
      ) : (
        "Sim, excluir"
      )}
    </button>
  );
}

/**
 * Two-step inline delete confirmation. The first click arms the action and
 * reveals an explicit confirm/cancel pair, so deletion is deliberate without
 * a jarring native dialog.
 */
export function ConfirmDeleteButton({
  children,
  confirmMessage,
}: {
  children: React.ReactNode;
  confirmMessage: string;
}) {
  const [armed, setArmed] = useState(false);

  if (!armed) {
    return (
      <button
        className="clip-notch focus-ring inline-flex min-h-10 items-center gap-2 border border-accent-border px-3 text-xs font-semibold text-accent transition duration-200 hover:bg-accent-soft active:scale-[0.985]"
        onClick={() => setArmed(true)}
        type="button"
      >
        <Trash2 size={14} aria-hidden="true" />
        {children}
      </button>
    );
  }

  return (
    <div className="flex flex-wrap items-center gap-2 rounded-md border border-accent-border bg-accent-soft px-3 py-2">
      <span className="text-xs leading-5 text-text-secondary">{confirmMessage}</span>
      <div className="flex items-center gap-2">
        <ConfirmSubmit />
        <button
          className="focus-ring inline-flex min-h-10 items-center rounded-md px-3 text-xs font-semibold text-text-muted transition duration-200 hover:text-text-primary"
          onClick={() => setArmed(false)}
          type="button"
        >
          Cancelar
        </button>
      </div>
    </div>
  );
}
