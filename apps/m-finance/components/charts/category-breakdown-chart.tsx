"use client";

import { Bar, BarChart, Cell, LabelList, Tooltip, XAxis, YAxis } from "recharts";
import { CurrencyTooltip } from "@/components/charts/chart-tooltip";
import { useChartWidth } from "@/components/charts/use-chart-width";
import { InlineEmpty } from "@/components/ui/inline-empty";
import {
  categoryLabelReserve,
  toCategorySlices,
  type CategorySlice,
} from "@/lib/calculations/charts/categories";
import { formatCurrency } from "@/lib/formatters/currency";
import { CHART_CURSOR_FILL, CHART_PALETTE, COLORS } from "@/lib/ui/colors";

export type CategoryDatum = { name: string; value: number };

export function CategoryBreakdownChart({ data }: { data: CategoryDatum[] }) {
  const { ref, width } = useChartWidth();
  const slices = toCategorySlices(data);

  if (slices.length === 0) {
    return <InlineEmpty>Sem contas categorizadas neste mês.</InlineEmpty>;
  }

  const height = Math.max(120, slices.length * 44);
  const labelReserve = categoryLabelReserve(slices.map(sliceLabelText));

  return (
    <div className="w-full" ref={ref}>
      {width > 0 ? (
        <BarChart
          barCategoryGap={10}
          data={slices}
          height={height}
          layout="vertical"
          // Espaço à direita para o rótulo de valor não ser cortado. A reserva
          // vem do rótulo mais largo da série: um número fixo cabia no valor de
          // quem escreveu e cortava o `%` a 375px.
          margin={{ left: 0, right: labelReserve, top: 4, bottom: 4 }}
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
          <Bar dataKey="value" isAnimationActive={false} name="Total" radius={[0, 4, 4, 0]}>
            {slices.map((slice, index) => (
              <Cell fill={CHART_PALETTE[index % CHART_PALETTE.length]} key={slice.name} />
            ))}
            <LabelList
              content={(props) => renderSliceLabel(slices, props as SliceLabelProps)}
              dataKey="value"
              position="right"
            />
          </Bar>
        </BarChart>
      ) : null}
    </div>
  );
}

/**
 * O rótulo como texto corrido, para medir.
 *
 * `renderSliceLabel` desenha o mesmo conteúdo em dois tons — e um `<tspan>` não
 * se mede por `length`. As duas funções precisam concordar: mudando o formato
 * de uma, mudar a outra, ou a reserva volta a cortar o fim da linha.
 */
function sliceLabelText(slice: CategorySlice) {
  return `${formatCurrency(slice.value)} · ${slice.percent}%`;
}

type SliceLabelProps = {
  x?: number | string;
  y?: number | string;
  width?: number | string;
  height?: number | string;
  index?: number;
};

/**
 * Valor e proporção no fim da barra.
 *
 * `LabelList` com `dataKey` só desenha o número cru. O rótulo aqui é composto
 * — `R$ 1.234,00 · 32%` — e o percentual vem pronto de `toCategorySlices`, que
 * já garantiu que a coluna soma 100.
 *
 * Função de renderização e não componente: o `content` do `LabelList` entrega
 * a posição da barra e o índice, não a linha do dado, então a fatia precisa vir
 * de fora por parâmetro. Declarar um componente dentro do render para fechar
 * sobre `slices` é o que a regra `react-hooks/static-components` proíbe.
 */
function renderSliceLabel(slices: CategorySlice[], { x, y, width, height, index }: SliceLabelProps) {
  const slice = slices[index ?? -1];
  if (!slice) return null;

  const left = Number(x ?? 0) + Number(width ?? 0) + 8;
  const middle = Number(y ?? 0) + Number(height ?? 0) / 2;

  return (
    <text dominantBaseline="middle" fill={COLORS.textSecondary} fontSize={12} x={left} y={middle}>
      {formatCurrency(slice.value)}
      <tspan fill={COLORS.muted}> · {slice.percent}%</tspan>
    </text>
  );
}
