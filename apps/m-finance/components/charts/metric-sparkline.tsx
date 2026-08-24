"use client";

import { Line, LineChart } from "recharts";
import { useChartWidth } from "@/components/charts/use-chart-width";
import { COLORS } from "@/lib/ui/colors";

/**
 * Tendência dentro do card de métrica.
 *
 * Sem eixo, sem grade, sem tooltip e sem ponto: o card já diz o valor exato, e
 * o que falta é a direção. Um sparkline que ganha eixo vira gráfico, e aí
 * compete com o número que ele deveria acompanhar.
 *
 * Menos de dois pontos não desenha nada — uma linha de um ponto é ruído com
 * aparência de informação.
 *
 * `aria-hidden` porque o valor já está no card em texto. Um leitor de tela
 * lendo o mesmo dado duas vezes, a segunda como gráfico sem rótulo, atrapalha.
 */
export function MetricSparkline({
  points,
  tone = "neutral",
}: {
  /** Do mais antigo para o mais novo. */
  points: number[];
  tone?: "accent" | "neutral";
}) {
  const { ref, width } = useChartWidth();

  if (points.length < 2) return null;

  const data = points.map((value, index) => ({ index, value }));

  return (
    <div aria-hidden="true" className="mt-3 h-10 w-full" ref={ref}>
      {width > 0 ? (
        <LineChart
          data={data}
          height={40}
          margin={{ left: 0, right: 0, top: 4, bottom: 4 }}
          width={width}
        >
          <Line
            dataKey="value"
            dot={false}
            isAnimationActive={false}
            stroke={tone === "accent" ? COLORS.accent : COLORS.muted}
            strokeWidth={1}
            type="monotone"
          />
        </LineChart>
      ) : null}
    </div>
  );
}
