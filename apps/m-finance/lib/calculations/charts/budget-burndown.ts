export type BurndownPoint = {
  day: number;
  /** Acumulado do dia 1 até este dia. */
  spentCents: number;
  /** O mesmo em todos os pontos — é o que desenha a linha reta do teto. */
  limitCents: number;
};

/**
 * O gasto acumulado dia a dia contra o teto.
 *
 * A barra de progresso do `budget-card` responde "quanto do teto já foi", e é
 * uma resposta sem tempo dentro. Estourar no dia 8 e estourar no dia 28 são
 * situações diferentes, e só o acumulado no eixo do mês separa as duas.
 */
export function toBudgetBurndown(
  items: { dueDate: string; amountCents: number }[],
  limitCents: number,
  year: number,
  month: number,
): BurndownPoint[] {
  const days = new Date(year, month, 0).getDate();
  const perDay = new Array<number>(days).fill(0);

  for (const item of items) {
    // Manual e não `new Date(iso)`: a string é interpretada como UTC, e no
    // fuso do Brasil o lançamento cai um dia antes.
    const [itemYear, itemMonth, itemDay] = item.dueDate.split("-").map(Number);
    if (itemYear !== year || itemMonth !== month) continue;
    if (!Number.isFinite(itemDay) || itemDay < 1 || itemDay > days) continue;
    perDay[itemDay - 1] += item.amountCents;
  }

  let running = 0;
  return perDay.map((value, index) => {
    running += value;
    return { day: index + 1, spentCents: running, limitCents };
  });
}

/**
 * O primeiro dia em que o acumulado ultrapassa o teto, ou `null` se o mês
 * inteiro couber. Empatar com o limite não é ultrapassar.
 */
export function crossingDay(points: BurndownPoint[]): number | null {
  const crossed = points.find((point) => point.spentCents > point.limitCents);
  return crossed ? crossed.day : null;
}
