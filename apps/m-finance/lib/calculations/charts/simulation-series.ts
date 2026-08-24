import { monthName, type SimulationMonth } from "@/lib/calculations/simulator";

export type SimulationPoint = {
  /** `ago/26`, no mesmo formato dos rótulos do histórico. */
  label: string;
  semCompra: number;
  comCompra: number;
};

/**
 * A projeção do simulador como duas linhas.
 *
 * O `recommendation` diz em texto que a compra cabe ou não cabe. Duas linhas
 * dizem **quando** ela deixa de caber, que é a informação que decide entre
 * comprar agora e comprar em três meses.
 */
export function toSimulationSeries(months: SimulationMonth[]): SimulationPoint[] {
  return months.map((month) => ({
    label: `${monthName(month.month).slice(0, 3)}/${String(month.year).slice(2)}`,
    semCompra: month.baselineRemainingCents,
    comCompra: month.remainingWithCents,
  }));
}

/**
 * O primeiro mês em que a sobra com a compra fica negativa, ou `null` se ela
 * aguentar a projeção inteira. Zerar não é ficar negativo.
 */
export function firstNegativeMonth(points: SimulationPoint[]): string | null {
  const breaking = points.find((point) => point.comCompra < 0);
  return breaking ? breaking.label : null;
}
