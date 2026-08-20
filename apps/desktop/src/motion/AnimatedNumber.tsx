import { useEffect, useRef, useState } from "react";
import { interpolateNumber } from "../motion";

export interface AnimatedNumberProps {
  value: number;
  duration?: number;
  decimals?: number;
  prefix?: string;
  suffix?: string;
  formatFn?: (val: number) => string;
  className?: string;
}

/**
 * AnimatedNumber (React Bits Count Up adaptado para M/OS)
 *
 * Animação fluida e tabular para métricas, tempos, horas e contagens.
 * Sem re-renderings excessivos e com respeito a reduced motion.
 */
export function AnimatedNumber({
  value,
  duration = 320,
  decimals = 0,
  prefix = "",
  suffix = "",
  formatFn,
  className = "",
}: AnimatedNumberProps) {
  const [displayValue, setDisplayValue] = useState(value);
  const startValRef = useRef(value);
  const startTimeRef = useRef<number | null>(null);
  const rafRef = useRef<number | null>(null);

  useEffect(() => {
    const prefersReduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (prefersReduced || duration <= 0) {
      setDisplayValue(value);
      startValRef.current = value;
      return;
    }

    const startVal = startValRef.current;
    const targetVal = value;

    if (startVal === targetVal) {
      setDisplayValue(targetVal);
      return;
    }

    startTimeRef.current = performance.now();

    const tick = (now: number) => {
      const elapsed = now - (startTimeRef.current ?? now);
      const progress = Math.min(1, elapsed / duration);
      const current = interpolateNumber(startVal, targetVal, progress);

      setDisplayValue(current);

      if (progress < 1) {
        rafRef.current = requestAnimationFrame(tick);
      } else {
        startValRef.current = targetVal;
      }
    };

    rafRef.current = requestAnimationFrame(tick);

    return () => {
      if (rafRef.current) cancelAnimationFrame(rafRef.current);
    };
  }, [value, duration]);

  const formatted = formatFn
    ? formatFn(displayValue)
    : decimals > 0
    ? displayValue.toFixed(decimals)
    : Math.round(displayValue).toString();

  return (
    <span className={`animated-number ${className}`.trim()} style={{ fontVariantNumeric: "tabular-nums" }}>
      {prefix}
      {formatted}
      {suffix}
    </span>
  );
}
