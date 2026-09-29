import { summarizeCardInstallments } from "@/lib/calculations/card-installments";
import { averageOfRecent } from "@/lib/calculations/forecast";
import { addMonths, monthKey, sameMonth } from "@/lib/finance-intelligence/dates";
import { monthOverview } from "@/lib/finance-intelligence/kernel/overview";
import { sanitizeLabel } from "@/lib/finance-intelligence/sanitize";
import type { FinanceSnapshot } from "@/lib/finance-intelligence/types";

export const MAX_SERIES_PER_CARD = 8;

/**
 * Onde cada cartão está: o que já foi cobrado, o que vem, e quanto disso é
 * parcela que já não dá para desfazer.
 *
 * `dataNotAvailable` existe por uma pergunta específica — "qual meu limite?".
 * O M-Finance não guarda limite de cartão, e a resposta honesta é dizer isso;
 * deduzir um limite pelo tamanho da fatura seria inventar um número.
 */
export function cardExposure(snapshot: FinanceSnapshot, options: { cardId?: string | null; months: number }) {
  const cards = snapshot.cards.filter(
    (card) => (options.cardId ? card.id === options.cardId : card.isActive),
  );
  const futureMonths = Array.from({ length: options.months }, (_, offset) =>
    addMonths(snapshot.current, offset),
  );
  const overviews = futureMonths.map((parts) => monthOverview(snapshot, parts));

  return {
    dataNotAvailable: ["limite do cartão", "data de fechamento da fatura"],
    cards: cards.map((card) => {
      const invoices = snapshot.invoices.filter((row) => row.cardId === card.id);
      const history = Array.from({ length: options.months }, (_, index) =>
        addMonths(snapshot.current, -(index + 1)),
      )
        .reverse()
        .map((parts) => {
          const invoice = invoices.find((row) => sameMonth(row, parts));
          return {
            key: monthKey(parts),
            amountCents: invoice?.amountCents ?? 0,
            status: invoice?.status ?? null,
          };
        });

      const forecast = overviews.map((overview) => {
        const line = overview.invoices.lines.find((row) => row.cardId === card.id);
        return {
          key: overview.key,
          amountCents: line?.amountCents ?? 0,
          source: line?.source ?? "none",
          status: line?.status ?? null,
          invoiceId: line?.invoiceId ?? null,
          dueDate: line?.dueDate ?? null,
        };
      });

      const expenses = snapshot.cardExpenses.filter((row) => row.cardId === card.id);
      const series = summarizeCardInstallments(expenses, snapshot.current);
      const futureInstallmentsCents = expenses
        .filter((row) => row.installmentId && monthKey(row) > monthKey(snapshot.current))
        .reduce((sum, row) => sum + row.amountCents, 0);

      const currentInvoice = forecast[0]?.amountCents ?? 0;
      const previous = history[history.length - 1]?.amountCents ?? 0;
      const { averageCents, count } = averageOfRecent(
        invoices.map((row) => ({ month: row.month, year: row.year, amountCents: row.amountCents })),
        snapshot.current,
      );

      return {
        id: card.id,
        name: sanitizeLabel(card.name),
        cardType: card.cardType,
        dueDay: card.dueDay,
        isActive: card.isActive,
        history,
        forecast,
        trend: {
          currentCents: currentInvoice,
          previousCents: previous,
          deltaVsPreviousCents: currentInvoice - previous,
          averageLast3Cents: averageCents,
          averageBasis: count,
          deltaVsAverageCents: count > 0 ? currentInvoice - averageCents : null,
        },
        installments: {
          liveSeries: series.slice(0, MAX_SERIES_PER_CARD).map((row) => ({
            installmentId: row.installmentId,
            description: sanitizeLabel(row.description),
            installmentCents: row.installmentCents,
            currentNumber: row.currentNumber,
            total: row.installmentTotal,
            remainingCents: row.remainingCents,
            endsIn: monthKey(row.lastMonth),
          })),
          seriesCount: series.length,
          futureInstallmentsCents,
        },
      };
    }),
  };
}
