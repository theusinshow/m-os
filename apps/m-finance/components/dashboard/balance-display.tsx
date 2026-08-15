"use client";

import { useSyncExternalStore } from "react";
import { Eye, EyeOff } from "lucide-react";
import { CurrencyCountUp } from "@/components/ui/currency-count-up";

const STORAGE_KEY = "mf-balance-hidden";

const listeners = new Set<() => void>();

function subscribe(callback: () => void) {
  listeners.add(callback);
  return () => {
    listeners.delete(callback);
  };
}

function getSnapshot() {
  return window.localStorage.getItem(STORAGE_KEY) === "1";
}

// Mask on the server and during hydration: the preference is unknown until the
// client reads localStorage, and starting masked means a hidden balance never
// flashes. useSyncExternalStore keeps server/client in sync (no hydration warn).
function getServerSnapshot() {
  return true;
}

function setHiddenPreference(next: boolean) {
  window.localStorage.setItem(STORAGE_KEY, next ? "1" : "0");
  listeners.forEach((listener) => listener());
}

/**
 * Hero figure with a privacy toggle persisted in localStorage.
 */
export function BalanceDisplay({
  cents,
  label = "Sobra prevista",
}: {
  cents: number;
  label?: string;
}) {
  const masked = useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);

  function toggle() {
    setHiddenPreference(!masked);
  }

  return (
    <div>
      <div className="flex items-center gap-2">
        <p className="text-sm font-medium uppercase tracking-[0.16em] text-text-muted">{label}</p>
        <button
          aria-label={masked ? "Mostrar valor" : "Esconder valor"}
          className="focus-ring inline-flex h-7 w-7 items-center justify-center rounded-md text-text-muted transition duration-200 hover:bg-background-elevated hover:text-text-primary"
          onClick={toggle}
          type="button"
        >
          {masked ? <EyeOff size={15} aria-hidden="true" /> : <Eye size={15} aria-hidden="true" />}
        </button>
      </div>
      <h1 className="mt-3 text-5xl font-semibold leading-none text-text-primary md:text-6xl">
        {masked ? (
          <span className="num text-text-muted">R$ ••••••</span>
        ) : (
          <CurrencyCountUp cents={cents} />
        )}
      </h1>
    </div>
  );
}
