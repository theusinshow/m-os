"use client";

import {
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ReferenceLine,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { CurrencyTooltip } from "@/components/charts/chart-tooltip";
import { useChartWidth } from "@/components/charts/use-chart-width";
import { InlineEmpty } from "@/components/ui/inline-empty";
import {
  firstNegativeMonth,
  toSimulationSeries,
} from "@/lib/calculations/charts/simulation-series";
import type { SimulationMonth } from "@/lib/calculations/simulator";
import { formatCurrencyCompact } from "@/lib/formatters/currency";
import { CHART_CURSOR_STROKE, CHART_GRID, COLORS } from "@/lib/ui/colors";

export function SimulationProjectionChart({ months }: { months: SimulationMonth[] }) {
  const { ref, width } = useChartWidth();
  const points = toSimulationSeries(months);
  const breaking = firstNegativeMonth(points);

  if (points.length < 2) {
    return <InlineEmpty>A projeção precisa de pelo menos dois meses.</InlineEmpty>;
  }

  return (
    <div>
      <div className="w-full" ref={ref}>
        {width > 0 ? (
          <LineChart
            data={points}
            height={200}
            margin={{ left: 0, right: 8, top: 8, bottom: 0 }}
            width={width}
          >
            <CartesianGrid stroke={CHART_GRID} vertical={false} />
            <XAxis
              axisLine={false}
              dataKey="label"
              tick={{ fill: COLORS.muted, fontSize: 11 }}
              tickLine={false}
            />
            <YAxis
              axisLine={false}
              tick={{ fill: COLORS.muted, fontSize: 11 }}
              tickFormatter={formatCurrencyCompact}
              tickLine={false}
              width={64}
            />
            {/* O zero é o assunto: onde a linha de baixo o cruza, a compra
                deixou de caber. */}
            <ReferenceLine stroke={COLORS.negative} strokeDasharray="4 4" y={0} />
            <Tooltip content={<CurrencyTooltip />} cursor={{ stroke: CHART_CURSOR_STROKE }} />
            <Legend
              formatter={(value) => (
                <span style={{ color: COLORS.muted, fontSize: 12 }}>{value}</span>
              )}
              iconType="plainline"
            />
            <Line
              dataKey="semCompra"
              dot={false}
              isAnimationActive={false}
              name="Sem a compra"
              stroke={COLORS.muted}
              strokeWidth={1}
              type="monotone"
            />
            <Line
              dataKey="comCompra"
              dot={false}
              isAnimationActive={false}
              name="Com a compra"
              stroke={breaking ? COLORS.negative : COLORS.accent}
              strokeWidth={2}
              type="monotone"
            />
          </LineChart>
        ) : null}
      </div>
      <p className="mt-3 text-xs text-text-muted">
        {breaking
          ? `A sobra fica negativa em ${breaking}.`
          : "A sobra aguenta a projeção inteira."}
      </p>
    </div>
  );
}
