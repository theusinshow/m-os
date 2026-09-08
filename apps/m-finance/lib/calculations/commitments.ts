type InvoiceShare = {
  amountCents: number;
  cardType: "personal" | "business";
};

/**
 * Quanto do mês em faturas é da empresa e quanto é seu.
 *
 * O schema já distingue cartão pessoal de PJ (`credit_cards.card_type`), mas
 * nenhuma tela somava os dois separadamente — um Nubank PJ de R$ 1.300 entrava
 * no mesmo balde do gasto pessoal, e o "quanto sobra" ficava R$ 1.300 mais
 * pessimista do que a vida real.
 */
export function splitPersonalBusiness(invoices: InvoiceShare[]) {
  const personalCents = invoices
    .filter((invoice) => invoice.cardType === "personal")
    .reduce((total, invoice) => total + invoice.amountCents, 0);
  const businessCents = invoices
    .filter((invoice) => invoice.cardType === "business")
    .reduce((total, invoice) => total + invoice.amountCents, 0);
  const totalCents = personalCents + businessCents;

  return {
    personalCents,
    businessCents,
    totalCents,
    businessPercent: totalCents === 0 ? 0 : Math.round((businessCents / totalCents) * 100),
  };
}

type InstallmentBill = {
  name: string;
  amountCents: number;
  seriesId: string | null;
  seriesNumber: number | null;
  seriesTotal: number | null;
  dueDate: string;
  status: "pending" | "paid" | "overdue";
};

export type InstallmentSeries = {
  seriesId: string;
  name: string;
  installmentCents: number;
  paidCount: number;
  remainingCount: number;
  remainingCents: number;
  seriesTotal: number;
  lastDueDate: string;
};

/**
 * O que um parcelamento ainda vai custar, somado.
 *
 * O app mostrava a parcela do mês e nada mais: um financiamento de 22× R$ 700
 * aparecia como "R$ 700" e os R$ 14.700 que faltavam não existiam em tela
 * nenhuma. A contagem sai de `seriesTotal - pagas` em vez do número de linhas
 * carregadas, para a conta não depender de quantos meses estão em memória.
 */
export function summarizeInstallments(bills: InstallmentBill[]): InstallmentSeries[] {
  const bySeries = new Map<string, InstallmentBill[]>();

  for (const bill of bills) {
    if (!bill.seriesId || !bill.seriesTotal || !bill.seriesNumber) continue;
    const rows = bySeries.get(bill.seriesId) ?? [];
    rows.push(bill);
    bySeries.set(bill.seriesId, rows);
  }

  return [...bySeries.entries()]
    .map(([seriesId, rows]) => {
      const sorted = [...rows].sort((a, b) => (a.seriesNumber ?? 0) - (b.seriesNumber ?? 0));
      const first = sorted[0];
      const seriesTotal = first.seriesTotal as number;
      const paidCount = sorted.filter((row) => row.status === "paid").length;
      const remainingCount = Math.max(seriesTotal - paidCount, 0);

      return {
        seriesId,
        name: first.name,
        installmentCents: first.amountCents,
        paidCount,
        remainingCount,
        remainingCents: remainingCount * first.amountCents,
        seriesTotal,
        lastDueDate: sorted.reduce(
          (latest, row) => (row.dueDate > latest ? row.dueDate : latest),
          sorted[0].dueDate,
        ),
      };
    })
    .filter((series) => series.remainingCount > 0)
    .sort((a, b) => b.remainingCents - a.remainingCents);
}
