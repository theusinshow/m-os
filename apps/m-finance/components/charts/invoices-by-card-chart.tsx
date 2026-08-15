"use client";

import { Bar, BarChart, Cell, Tooltip, XAxis, YAxis } from "recharts";
import { CurrencyTooltip } from "@/components/charts/chart-tooltip";
import { useChartWidth } from "@/components/charts/use-chart-width";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { CHART_CURSOR_FILL, COLORS } from "@/lib/ui/colors";

export type InvoiceCardDatum = { name: string; value: number; isBusiness: boolean };

export function InvoicesByCardChart({ data }: { data: InvoiceCardDatum[] }) {
  const { ref, width } = useChartWidth();
  const sorted = [...data].filter((d) => d.value > 0).sort((a, b) => b.value - a.value);

  if (sorted.length === 0) {
    return <InlineEmpty>Nenhuma fatura lançada neste mês.</InlineEmpty>;
  }

  return (
    <div className="w-full" ref={ref}>
      {width > 0 ? (
        <BarChart
          data={sorted}
          height={220}
          margin={{ left: 0, right: 8, top: 8, bottom: 0 }}
          width={width}
        >
          <XAxis
            axisLine={false}
            dataKey="name"
            tick={{ fill: COLORS.muted, fontSize: 12 }}
            tickLine={false}
          />
          <YAxis hide />
          <Tooltip content={<CurrencyTooltip />} cursor={{ fill: CHART_CURSOR_FILL }} />
          <Bar dataKey="value" name="Fatura" radius={[4, 4, 0, 0]}>
            {sorted.map((entry) => (
              // PJ cards in the warm fair tone, personal in brand red, matching the cards UI.
              <Cell fill={entry.isBusiness ? COLORS.fair : COLORS.accent} key={entry.name} />
            ))}
          </Bar>
        </BarChart>
      ) : null}
    </div>
  );
}
