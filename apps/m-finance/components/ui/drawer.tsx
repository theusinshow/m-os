"use client";

import { useEffect, useId, useRef } from "react";
import { createPortal } from "react-dom";
import { X } from "lucide-react";
import { TriangleMark } from "@/components/brand/triangle-mark";
import { cn } from "@/lib/utils";

const FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), summary, [tabindex]:not([tabindex="-1"])';

/**
 * Drawer acessível: painel lateral direito no desktop, bottom sheet no mobile.
 * - Escape fecha, foco fica preso dentro, foco retorna ao gatilho ao fechar.
 * - Clique no backdrop fecha. Scroll do body travado enquanto aberto.
 */
export function Drawer({
  open,
  onClose,
  title,
  description,
  children,
  footer,
}: {
  open: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  children: React.ReactNode;
  footer?: React.ReactNode;
}) {
  const panelRef = useRef<HTMLDivElement>(null);
  const restoreFocusRef = useRef<HTMLElement | null>(null);
  const titleId = useId();

  useEffect(() => {
    if (!open) return;

    restoreFocusRef.current =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;

    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";

    const panel = panelRef.current;
    const firstFocusable = panel?.querySelector<HTMLElement>(FOCUSABLE_SELECTOR);
    (firstFocusable ?? panel)?.focus();

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
        return;
      }
      if (event.key !== "Tab" || !panel) return;

      const focusables = Array.from(panel.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
        (el) => el.offsetParent !== null,
      );
      if (focusables.length === 0) {
        event.preventDefault();
        panel.focus();
        return;
      }
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      const active = document.activeElement;

      if (event.shiftKey && (active === first || active === panel)) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && active === last) {
        event.preventDefault();
        first.focus();
      }
    }

    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.body.style.overflow = previousOverflow;
      document.removeEventListener("keydown", handleKeyDown);
      restoreFocusRef.current?.focus();
    };
  }, [open, onClose]);

  // O drawer só abre após interação do usuário, então o portal sempre
  // encontra o document pronto — sem estado de "mounted".
  if (!open || typeof document === "undefined") return null;

  return createPortal(
    <div className="fixed inset-0 z-[70]">
      <div
        aria-hidden="true"
        className="drawer-backdrop absolute inset-0 bg-background-primary/70 backdrop-blur-sm"
        onClick={onClose}
      />
      <div
        aria-labelledby={titleId}
        aria-modal="true"
        className={cn(
          "absolute flex flex-col border-border-subtle bg-background-secondary shadow-2xl shadow-black/50",
          "inset-x-0 bottom-0 max-h-[88dvh] rounded-t-2xl border-t drawer-panel-up",
          "sm:inset-y-0 sm:right-0 sm:left-auto sm:h-full sm:w-[27rem] sm:max-w-full sm:rounded-none sm:border-t-0 sm:border-l sm:drawer-panel-right",
        )}
        ref={panelRef}
        role="dialog"
        tabIndex={-1}
      >
        <div
          aria-hidden="true"
          className="mx-auto mt-3 h-1 w-10 shrink-0 rounded-full bg-border-strong sm:hidden"
        />
        <header className="flex items-start justify-between gap-4 border-b border-border-subtle px-5 py-4">
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <TriangleMark className="text-accent" size={8} variant="solid" />
              <p
                className="truncate font-display text-base font-semibold text-text-primary"
                id={titleId}
              >
                {title}
              </p>
            </div>
            {description ? (
              <p className="mt-1 text-xs leading-5 text-text-muted">{description}</p>
            ) : null}
          </div>
          <button
            aria-label="Fechar painel"
            className="focus-ring -mr-1 shrink-0 rounded-md p-1.5 text-text-muted transition hover:bg-background-hover hover:text-text-primary"
            onClick={onClose}
            type="button"
          >
            <X size={18} aria-hidden="true" />
          </button>
        </header>
        <div className="flex-1 overflow-y-auto px-5 py-5">{children}</div>
        {footer ? (
          <footer className="border-t border-border-subtle px-5 py-4 pb-[calc(1rem+env(safe-area-inset-bottom))]">
            {footer}
          </footer>
        ) : null}
      </div>
    </div>,
    document.body,
  );
}
