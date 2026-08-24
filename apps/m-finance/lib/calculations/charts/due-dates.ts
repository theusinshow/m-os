export type DueDateBucket = {
  /** 1 a 28/29/30/31. */
  day: number;
  cents: number;
  /** 0 a 1, relativo ao dia mais pesado do mês. */
  intensity: number;
};

/**
 * Quanto vence em cada dia do mês.
 *
 * A intensidade é relativa ao dia mais pesado, e não a um teto absoluto: o
 * gráfico responde "onde este mês aperta", não "este mês é pior que março".
 * Comparar meses é trabalho do histórico.
 */
export function toDueDateBuckets(
  items: { dueDate: string; amountCents: number }[],
  year: number,
  month: number,
): DueDateBucket[] {
  const days = daysInMonth(year, month);
  const cents = new Array<number>(days).fill(0);

  for (const item of items) {
    const parsed = parseIsoDate(item.dueDate);
    if (!parsed) continue;
    if (parsed.year !== year || parsed.month !== month) continue;
    if (parsed.day < 1 || parsed.day > days) continue;
    cents[parsed.day - 1] += item.amountCents;
  }

  const heaviest = Math.max(...cents, 0);

  return cents.map((value, index) => ({
    day: index + 1,
    cents: value,
    intensity: heaviest === 0 ? 0 : value / heaviest,
  }));
}

/** Dia 0 do mês seguinte é o último dia deste. Cobre bissexto sem tabela. */
function daysInMonth(year: number, month: number) {
  return new Date(year, month, 0).getDate();
}

/**
 * `YYYY-MM-DD` na mão, de propósito.
 *
 * `new Date("2026-08-10")` é interpretado como meia-noite UTC, que no fuso do
 * Brasil é dia 9. Num heatmap por dia isso desloca a coluna inteira.
 */
function parseIsoDate(value: string) {
  const [year, month, day] = value.split("-").map(Number);
  if (!Number.isFinite(year) || !Number.isFinite(month) || !Number.isFinite(day)) {
    return null;
  }
  return { year, month, day };
}
