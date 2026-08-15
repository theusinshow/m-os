"use client";

import { CartesianGrid, Legend, Line, LineChart, Tooltip, XAxis, YAxis } from "recharts";
import { CurrencyTooltip } from "@/components/charts/chart-tooltip";
import { useChartWidth } from "@/components/charts/use-chart-width";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { CHART_CURSOR_STROKE, CHART_GRID, COLORS } from "@/lib/ui/colors";

export type TrendDatum = {
  label: string;
  receita: number;
  comprometido: number;
  sobra: number;
};

const SERIES = [
  { key: "receita", name: "Receita", color: COLORS.positive },
  { key: "comprometido", name: "Comprometido", color: COLORS.tight },
  { key: "sobra", name: "Sobra", color: COLORS.accent },
] as const;

export function HistoryTrendChart({ data }: { data: TrendDatum[] }) {
  const { ref, width } = useChartWidth();

  if (data.length < 2) {
    return (
      <InlineEmpty>Salve pelo menos dois meses para ver a evolução.</InlineEmpty>
    );
  }

  return (
    <div className="w-full" ref={ref}>
      {width > 0 ? (
        <LineChart
          data={data}
          height={240}
          margin={{ left: 0, right: 8, top: 8, bottom: 0 }}
          width={width}
        >
          <CartesianGrid stroke={CHART_GRID} vertical={false} />
          <XAxis
            axisLine={false}
            dataKey="label"
            tick={{ fill: COLORS.muted, fontSize: 12 }}
            tickLine={false}
          />
          <YAxis hide />
          <Tooltip content={<CurrencyTooltip />} cursor={{ stroke: CHART_CURSOR_STROKE }} />
          <Legend
            formatter={(value) => <span style={{ color: COLORS.muted, fontSize: 12 }}>{value}</span>}
            iconType="plainline"
          />
          {SERIES.map((series) => (
            <Line
              dataKey={series.key}
              dot={false}
              key={series.key}
              name={series.name}
              stroke={series.color}
              strokeWidth={2}
              type="monotone"
            />
          ))}
        </LineChart>
      ) : null}
    </div>
  );
}
