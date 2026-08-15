"use client";

import { formatCurrency } from "@/lib/formatters/currency";

type TooltipEntry = {
  name?: string;
  value?: number | string;
  color?: string;
  dataKey?: string | number;
};

type CurrencyTooltipProps = {
  active?: boolean;
  payload?: TooltipEntry[];
  label?: string | number;
};

// Shared dark tooltip rendering each series as "name: R$ value". Typed locally
// because Recharts' exported TooltipProps shape varies across versions.
export function CurrencyTooltip({ active, payload, label }: CurrencyTooltipProps) {
  if (!active || !payload || payload.length === 0) return null;

  return (
    <div className="rounded-md border border-border-default bg-background-elevated px-3 py-2 text-xs shadow-lg">
      {label ? <p className="mb-1 font-semibold text-text-primary">{label}</p> : null}
      <ul className="space-y-0.5">
        {payload.map((entry, index) => (
          <li
            className="flex items-center gap-2 text-text-secondary"
            key={String(entry.dataKey ?? index)}
          >
            <span
              aria-hidden="true"
              className="tri-mark h-2 w-2"
              style={{ backgroundColor: entry.color }}
            />
            <span>{entry.name}</span>
            <span className="num ml-auto font-semibold text-text-primary">
              {formatCurrency(Number(entry.value ?? 0))}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}
