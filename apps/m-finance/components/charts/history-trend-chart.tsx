"use client";

import { CartesianGrid, Legend, Line, LineChart, Tooltip, XAxis, YAxis } from "recharts";
import { CurrencyTooltip } from "@/components/charts/chart-tooltip";
import { useChartWidth } from "@/components/charts/use-chart-width";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { formatCurrency, formatCurrencyCompact } from "@/lib/formatters/currency";
import { CHART_CURSOR_STROKE, CHART_GRID, COLORS } from "@/lib/ui/colors";

export type TrendDatum = {
  label: string;
  receita: number;
  comprometido: number;
  sobra: number;
};

/**
 * A ordem importa: `sobra` por último para desenhar por cima, e com o peso
 * visual que as outras duas não têm.
 *
 * A pergunta da tela é "a sobra está melhorando?". Três linhas de mesma
 * espessura e cor equivalente fazem o olho procurar qual delas responde; uma
 * linha em sódio sobre duas neutras finas já responde antes da leitura.
 */
const SERIES = [
  { key: "receita", name: "Receita", color: COLORS.muted, width: 1 },
  { key: "comprometido", name: "Comprometido", color: COLORS.textSecondary, width: 1 },
  { key: "sobra", name: "Sobra", color: COLORS.accent, width: 2 },
] as const;

export function HistoryTrendChart({ data }: { data: TrendDatum[] }) {
  const { ref, width } = useChartWidth();

  if (data.length < 2) {
    return <InlineEmpty>Salve pelo menos dois meses para ver a evolução.</InlineEmpty>;
  }

  const latest = data[data.length - 1];

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
          {/* Largura fixa: sem ela o eixo mede o rótulo mais largo de cada mês
              e a área de desenho pula de tamanho ao trocar de período. */}
          <YAxis
            axisLine={false}
            tick={{ fill: COLORS.muted, fontSize: 11 }}
            tickFormatter={formatCurrencyCompact}
            tickLine={false}
            width={64}
          />
          <Tooltip content={<CurrencyTooltip />} cursor={{ stroke: CHART_CURSOR_STROKE }} />
          {/* A legenda carrega o valor do último mês: sem ele ela é só um
              decodificador de cor, e obriga a voltar ao gráfico para saber
              quanto cada linha vale hoje. */}
          <Legend
            formatter={(value, entry) => {
              const key = String(
                (entry as { dataKey?: string | number } | undefined)?.dataKey ?? "",
              ) as keyof TrendDatum;
              const current = typeof latest[key] === "number" ? (latest[key] as number) : null;
              return (
                <span style={{ color: COLORS.muted, fontSize: 12 }}>
                  {value}
                  {current === null ? null : (
                    <span style={{ color: COLORS.textSecondary, marginLeft: 6 }}>
                      {formatCurrency(current)}
                    </span>
                  )}
                </span>
              );
            }}
            iconType="plainline"
          />
          {SERIES.map((series) => (
            <Line
              dataKey={series.key}
              dot={false}
              isAnimationActive={false}
              key={series.key}
              name={series.name}
              stroke={series.color}
              strokeWidth={series.width}
              type="monotone"
            />
          ))}
        </LineChart>
      ) : null}
    </div>
  );
}
