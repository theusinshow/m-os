"use client";

import { useEffect, useRef, useState, useTransition } from "react";
import { Check, ChevronDown, ChevronLeft, ChevronRight } from "lucide-react";
import { setActiveMonth } from "@/app/actions/active-month";
import { cn } from "@/lib/utils";

type Option = { value: string; label: string; isCurrent: boolean };

export function MonthSwitcher({
  options,
  activeValue,
}: {
  options: Option[];
  activeValue: string;
}) {
  const [open, setOpen] = useState(false);
  const [pending, startTransition] = useTransition();
  const triggerRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLUListElement>(null);

  // Acessibilidade: Escape fecha e devolve o foco ao gatilho; ao abrir,
  // o foco vai para a opção selecionada.
  useEffect(() => {
    if (!open) return;

    const selected = listRef.current?.querySelector<HTMLElement>('[aria-selected="true"]');
    (selected ?? listRef.current?.querySelector<HTMLElement>("button"))?.focus();

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        setOpen(false);
        triggerRef.current?.focus();
      }
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [open]);

  const activeIndex = Math.max(
    0,
    options.findIndex((option) => option.value === activeValue),
  );
  const active = options[activeIndex] ?? options[0];
  // options run newest-first, so the older month sits at a higher index.
  const olderValue = options[activeIndex + 1]?.value;
  const newerValue = options[activeIndex - 1]?.value;

  function go(value: string | undefined) {
    if (!value || pending) return;
    setOpen(false);
    startTransition(() => setActiveMonth(value));
  }

  return (
    <div className="flex w-full items-center gap-1.5 md:w-auto">
      <button
        aria-label="Mês anterior"
        className="focus-ring inline-flex h-9 w-9 items-center justify-center rounded-md border border-border-subtle text-text-muted transition duration-150 hover:border-border-default hover:text-text-primary disabled:opacity-40"
        disabled={!olderValue || pending}
        onClick={() => go(olderValue)}
        type="button"
      >
        <ChevronLeft size={16} />
      </button>

      <div className="relative min-w-0 flex-1 md:flex-none">
        <button
          aria-expanded={open}
          aria-haspopup="listbox"
          className="focus-ring flex min-h-9 w-full items-center justify-between gap-2 rounded-md border border-border-subtle px-3 text-left transition duration-150 hover:border-border-default md:w-auto"
          onClick={() => setOpen((value) => !value)}
          ref={triggerRef}
          type="button"
        >
          <span className="min-w-0">
            <span className="block text-[10px] font-medium uppercase tracking-[0.16em] text-text-muted">
              {active?.isCurrent ? "Mês atual" : "Mês"}
            </span>
            <span className="block truncate text-base font-semibold leading-tight text-text-primary">
              {active?.label}
            </span>
          </span>
          <ChevronDown
            className={cn("text-text-muted transition-transform duration-200", open && "rotate-180")}
            size={15}
          />
        </button>

        {open ? (
          <>
            <button
              aria-hidden="true"
              className="fixed inset-0 z-30 cursor-default"
              onClick={() => setOpen(false)}
              tabIndex={-1}
              type="button"
            />
            <ul
              className="absolute left-0 z-40 mt-2 max-h-72 w-full min-w-56 overflow-auto rounded-lg border border-border-default bg-background-card p-1.5 shadow-xl shadow-black/30 md:w-56"
              ref={listRef}
              role="listbox"
            >
              {options.map((option) => {
                const selected = option.value === active?.value;
                return (
                  <li key={option.value}>
                    <button
                      className={cn(
                        "focus-ring flex w-full items-center justify-between gap-2 rounded-md px-3 py-2 text-left text-sm transition duration-150",
                        selected
                          ? "bg-accent-soft text-accent"
                          : "text-text-secondary hover:bg-background-elevated hover:text-text-primary",
                      )}
                      onClick={() => go(option.value)}
                      role="option"
                      aria-selected={selected}
                      type="button"
                    >
                      <span className="flex items-center gap-2">
                        {option.label}
                        {option.isCurrent ? (
                          <span className="rounded-sm border border-border-subtle px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-[0.12em] text-text-muted">
                            atual
                          </span>
                        ) : null}
                      </span>
                      {selected ? <Check size={14} aria-hidden="true" /> : null}
                    </button>
                  </li>
                );
              })}
            </ul>
          </>
        ) : null}
      </div>

      <button
        aria-label="Próximo mês"
        className="focus-ring inline-flex h-9 w-9 items-center justify-center rounded-md border border-border-subtle text-text-muted transition duration-150 hover:border-border-default hover:text-text-primary disabled:opacity-40"
        disabled={!newerValue || pending}
        onClick={() => go(newerValue)}
        type="button"
      >
        <ChevronRight size={16} />
      </button>
    </div>
  );
}
