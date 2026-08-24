"use client";

import { Bar, BarChart, Cell, ReferenceLine, Tooltip, XAxis, YAxis } from "recharts";
import { useChartWidth } from "@/components/charts/use-chart-width";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { toWaterfallSteps, type WaterfallStep } from "@/lib/calculations/charts/waterfall";
import { formatCurrency, formatCurrencyCompact } from "@/lib/formatters/currency";
import { CHART_CURSOR_FILL, CHART_GRID, COLORS } from "@/lib/ui/colors";

export function MonthWaterfallChart({
  incomeCents,
  billsCents,
  invoicesCents,
}: {
  incomeCents: number;
  billsCents: number;
  invoicesCents: number;
}) {
  const { ref, width } = useChartWidth();
  const steps = toWaterfallSteps({ incomeCents, billsCents, invoicesCents });

  if (incomeCents === 0 && billsCents === 0 && invoicesCents === 0) {
    return <InlineEmpty>Cadastre receita e contas para ver o mês em cascata.</InlineEmpty>;
  }

  return (
    <div className="w-full" ref={ref}>
      {width > 0 ? (
        <BarChart
          data={steps}
          height={220}
          margin={{ left: 0, right: 8, top: 8, bottom: 0 }}
          width={width}
        >
          <XAxis
            axisLine={false}
            dataKey="label"
            tick={{ fill: COLORS.muted, fontSize: 12 }}
            tickLine={false}
          />
          <YAxis
            axisLine={false}
            tick={{ fill: COLORS.muted, fontSize: 11 }}
            tickFormatter={formatCurrencyCompact}
            tickLine={false}
            width={64}
          />
          <ReferenceLine stroke={CHART_GRID} y={0} />
          <Tooltip content={<WaterfallTooltip />} cursor={{ fill: CHART_CURSOR_FILL }} />
          {/* A base é a série que posiciona a barra e não se vê. */}
          <Bar dataKey="offset" fillOpacity={0} isAnimationActive={false} stackId="cascata" />
          <Bar dataKey="delta" isAnimationActive={false} radius={[3, 3, 0, 0]} stackId="cascata">
            {steps.map((step) => (
              <Cell fill={stepColor(step)} key={step.label} />
            ))}
          </Bar>
        </BarChart>
      ) : null}
    </div>
  );
}

/**
 * Cor por papel, dentro da rampa.
 *
 * Entrada usa o verde e saída a escala neutra porque `globals.css` registra
 * que dinheiro entrando e saindo continua colorido: é informação. O total vai
 * para o sódio quando sobra e para o vermelho quando falta — é o único ponto
 * do gráfico onde o acento do sistema se justifica.
 */
function stepColor(step: WaterfallStep) {
  if (step.kind === "in") return COLORS.positive;
  if (step.kind === "out") return COLORS.muted;
  return step.value < 0 ? COLORS.negative : COLORS.accent;
}

type WaterfallTooltipProps = {
  active?: boolean;
  payload?: { payload?: WaterfallStep }[];
};

/**
 * Tooltip próprio em vez do `CurrencyTooltip` compartilhado: ali cada série
 * vira uma linha, e aqui a série `offset` é andaime invisível que não deve
 * aparecer. O que importa é o `value` com sinal.
 */
function WaterfallTooltip({ active, payload }: WaterfallTooltipProps) {
  const step = payload?.[0]?.payload;
  if (!active || !step) return null;

  return (
    <div className="rounded-md border border-border-default bg-background-elevated px-3 py-2 text-xs shadow-lg">
      <p className="font-semibold text-text-primary">{step.label}</p>
      <p className="num mt-0.5 font-semibold text-text-secondary">
        {step.value < 0 ? "−" : ""}
        {formatCurrency(Math.abs(step.value))}
      </p>
    </div>
  );
}
