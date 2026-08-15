"use client";

import { useEffect, useRef, useState } from "react";
import { formatCurrency } from "@/lib/formatters/currency";

function prefersReducedMotion() {
  return (
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

/**
 * Counts a BRL value up to its final amount once, on mount. Used for the
 * dashboard hero metric so the primary number lands with intent, not flash.
 * Starts at 0 on the client (when motion is welcome) and animates up; SSR and
 * reduced-motion render the final value directly.
 */
export function CurrencyCountUp({
  cents,
  durationMs = 900,
}: {
  cents: number;
  durationMs?: number;
}) {
  const [value, setValue] = useState(() =>
    typeof window !== "undefined" && !prefersReducedMotion() ? 0 : cents,
  );
  const frame = useRef(0);

  useEffect(() => {
    if (prefersReducedMotion()) {
      return;
    }

    const start = performance.now();
    const tick = (now: number) => {
      const t = Math.min(1, (now - start) / durationMs);
      const eased = 1 - Math.pow(1 - t, 4);
      setValue(Math.round(cents * eased));
      if (t < 1) frame.current = requestAnimationFrame(tick);
    };
    frame.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame.current);
  }, [cents, durationMs]);

  return (
    <span className="num" suppressHydrationWarning>
      {formatCurrency(value)}
    </span>
  );
}
