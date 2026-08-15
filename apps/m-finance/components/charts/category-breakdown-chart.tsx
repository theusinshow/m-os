"use client";

import { Bar, BarChart, Cell, Tooltip, XAxis, YAxis } from "recharts";
import { CurrencyTooltip } from "@/components/charts/chart-tooltip";
import { useChartWidth } from "@/components/charts/use-chart-width";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { CHART_CURSOR_FILL, CHART_PALETTE, COLORS } from "@/lib/ui/colors";

export type CategoryDatum = { name: string; value: number };

export function CategoryBreakdownChart({ data }: { data: CategoryDatum[] }) {
  const { ref, width } = useChartWidth();
  const sorted = [...data].filter((d) => d.value > 0).sort((a, b) => b.value - a.value);

  if (sorted.length === 0) {
    return <InlineEmpty>Sem despesas categorizadas neste mês.</InlineEmpty>;
  }

  const height = Math.max(120, sorted.length * 44);

  return (
    <div className="w-full" ref={ref}>
      {width > 0 ? (
        <BarChart
          barCategoryGap={10}
          data={sorted}
          height={height}
          layout="vertical"
          margin={{ left: 0, right: 12, top: 4, bottom: 4 }}
          width={width}
        >
          <XAxis hide type="number" />
          <YAxis
            axisLine={false}
            dataKey="name"
            tick={{ fill: COLORS.muted, fontSize: 12 }}
            tickLine={false}
            type="category"
            width={96}
          />
          <Tooltip content={<CurrencyTooltip />} cursor={{ fill: CHART_CURSOR_FILL }} />
          <Bar dataKey="value" name="Total" radius={[0, 4, 4, 0]}>
            {sorted.map((entry, index) => (
              <Cell fill={CHART_PALETTE[index % CHART_PALETTE.length]} key={entry.name} />
            ))}
          </Bar>
        </BarChart>
      ) : null}
    </div>
  );
}
