"use client";

import {
  CartesianGrid,
  ReferenceLine,
  Scatter,
  ScatterChart,
  Tooltip,
  XAxis,
  YAxis,
  ZAxis,
} from "recharts";
import { useChartWidth } from "@/components/charts/use-chart-width";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { toGoalMatrix, type GoalMatrixPoint } from "@/lib/calculations/charts/goal-matrix";
import { formatCurrency } from "@/lib/formatters/currency";
import type { GoalWithProgress } from "@/lib/goals";
import { CHART_CURSOR_STROKE, CHART_GRID, COLORS } from "@/lib/ui/colors";

export function GoalPriorityMatrix({
  goals,
  today,
}: {
  goals: GoalWithProgress[];
  /** ISO `YYYY-MM-DD`, para o Server Component não passar `Date` cru. */
  today: string;
}) {
  const { ref, width } = useChartWidth();
  const [year, month, day] = today.split("-").map(Number);
  const { points, withoutDeadline } = toGoalMatrix(goals, new Date(year, month - 1, day));

  if (points.length === 0) {
    return (
      <InlineEmpty>
        Nenhuma meta com prazo. Defina um prazo para ver quais estão apertadas.
      </InlineEmpty>
    );
  }

  return (
    <div>
      <div className="w-full" ref={ref}>
        {width > 0 ? (
          <ScatterChart
            height={260}
            margin={{ left: 0, right: 16, top: 8, bottom: 16 }}
            width={width}
          >
            <CartesianGrid stroke={CHART_GRID} />
            {/* Invertido: prazo curto à esquerda, junto do "falta muito" no
                topo. O canto superior esquerdo vira o canto do risco. */}
            <XAxis
              axisLine={false}
              dataKey="daysLeft"
              name="Dias até o prazo"
              reversed
              tick={{ fill: COLORS.muted, fontSize: 11 }}
              tickLine={false}
              type="number"
              unit=" d"
            />
            <YAxis
              axisLine={false}
              dataKey="remainingPercent"
              domain={[0, 100]}
              name="Falta"
              tick={{ fill: COLORS.muted, fontSize: 11 }}
              tickLine={false}
              type="number"
              unit="%"
              width={44}
            />
            <ZAxis dataKey="remainingCents" range={[60, 400]} />
            {/* O prazo de hoje: à esquerda desta linha, a meta já venceu. */}
            <ReferenceLine stroke={COLORS.negative} strokeDasharray="3 3" x={0} />
            <Tooltip content={<GoalTooltip />} cursor={{ stroke: CHART_CURSOR_STROKE }} />
            <Scatter
              data={points}
              fill={COLORS.accent}
              fillOpacity={0.7}
              isAnimationActive={false}
              name="Metas"
            />
          </ScatterChart>
        ) : null}
      </div>

      {withoutDeadline.length > 0 ? (
        <p className="mt-3 text-xs text-text-muted">
          Sem prazo, fora do gráfico:{" "}
          <span className="text-text-secondary">
            {withoutDeadline.map((goal) => goal.name).join(", ")}
          </span>
          .
        </p>
      ) : null}
    </div>
  );
}

type GoalTooltipProps = {
  active?: boolean;
  payload?: { payload?: GoalMatrixPoint }[];
};

function GoalTooltip({ active, payload }: GoalTooltipProps) {
  const point = payload?.[0]?.payload;
  if (!active || !point) return null;

  return (
    <div className="rounded-md border border-border-default bg-background-elevated px-3 py-2 text-xs shadow-lg">
      <p className="font-semibold text-text-primary">{point.name}</p>
      <p className="num mt-0.5 text-text-secondary">
        faltam {formatCurrency(point.remainingCents)} ({point.remainingPercent}%)
      </p>
      <p className="num mt-0.5 text-text-muted">
        {point.daysLeft < 0
          ? `prazo venceu há ${Math.abs(point.daysLeft)} dia(s)`
          : `${point.daysLeft} dia(s) até o prazo`}
      </p>
    </div>
  );
}
