"use client";

import { CartesianGrid, Line, LineChart, ReferenceLine, Tooltip, XAxis, YAxis } from "recharts";
import { CurrencyTooltip } from "@/components/charts/chart-tooltip";
import { useChartWidth } from "@/components/charts/use-chart-width";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { crossingDay, toBudgetBurndown } from "@/lib/calculations/charts/budget-burndown";
import { formatCurrency, formatCurrencyCompact } from "@/lib/formatters/currency";
import { CHART_CURSOR_STROKE, CHART_GRID, COLORS } from "@/lib/ui/colors";

export function BudgetThresholdBand({
  items,
  limitCents,
  month,
  year,
}: {
  items: { dueDate: string; amountCents: number }[];
  limitCents: number;
  month: number;
  year: number;
}) {
  const { ref, width } = useChartWidth();
  const points = toBudgetBurndown(items, limitCents, year, month);
  const crossed = crossingDay(points);

  if (points[points.length - 1].spentCents === 0) {
    return <InlineEmpty>Nada lançado neste orçamento ainda.</InlineEmpty>;
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
              dataKey="day"
              // Um rótulo a cada cinco dias: 31 números seguidos viram borrão.
              interval={4}
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
            {/* A faixa de alerta do app é 80%, a mesma de `lib/budgets.ts`. */}
            <ReferenceLine
              stroke={COLORS.textSecondary}
              strokeDasharray="2 4"
              y={limitCents * 0.8}
            />
            <ReferenceLine stroke={COLORS.negative} strokeDasharray="4 4" y={limitCents} />
            <Tooltip content={<CurrencyTooltip />} cursor={{ stroke: CHART_CURSOR_STROKE }} />
            <Line
              dataKey="spentCents"
              dot={false}
              isAnimationActive={false}
              name="Gasto acumulado"
              stroke={crossed ? COLORS.negative : COLORS.accent}
              strokeWidth={2}
              type="monotone"
            />
          </LineChart>
        ) : null}
      </div>
      <p className="mt-3 text-xs text-text-muted">
        {crossed
          ? `Passou de ${formatCurrency(limitCents)} no dia ${crossed}.`
          : `O mês coube em ${formatCurrency(limitCents)}.`}
      </p>
    </div>
  );
}
