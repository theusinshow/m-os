import { and, eq, isNotNull, sql } from "drizzle-orm";
import { db } from "@/db/client";
import { creditCardExpenses, creditCardInvoices, creditCards, incomes, months } from "@/db/schema";
import {
  forecastCardMonth,
  forecastMainIncome,
  type CardForecastInputs,
  type MonthParts,
} from "@/lib/calculations/forecast";
import { buildMonthProjection, type MonthTotals } from "@/lib/calculations/projection";
import { getCurrentMonthParts, getMonthPartsAtOffset, getMonthTotalsForUser } from "@/lib/months";
import { derivePayableStatus } from "@/lib/status";

export type ForecastInputs = CardForecastInputs & {
  incomes: (MonthParts & { amountCents: number; incomeType: "main" | "extra" | "freelance" })[];
};

/**
 * Tudo que a estimativa precisa, de uma vez: cartões, faturas reais, parcelas
 * por mês e receitas. Poucas linhas — um uso pessoal tem dezenas, não milhares.
 */
export async function getForecastInputs(userId: string): Promise<ForecastInputs> {
  if (!db) {
    return { cards: [], invoices: [], installments: [], incomes: [] };
  }

  const [cardRows, invoiceRows, installmentRows, incomeRows] = await Promise.all([
    db
      .select({
        id: creditCards.id,
        name: creditCards.name,
        cardType: creditCards.cardType,
        dueDay: creditCards.dueDay,
        isActive: creditCards.isActive,
      })
      .from(creditCards)
      .where(eq(creditCards.userId, userId))
      .orderBy(creditCards.name),
    db
      .select({
        id: creditCardInvoices.id,
        cardId: creditCardInvoices.cardId,
        amountCents: creditCardInvoices.amountCents,
        dueDate: creditCardInvoices.dueDate,
        status: creditCardInvoices.status,
        month: months.month,
        year: months.year,
      })
      .from(creditCardInvoices)
      .innerJoin(months, eq(creditCardInvoices.monthId, months.id))
      .where(eq(creditCardInvoices.userId, userId)),
    db
      .select({
        cardId: creditCardExpenses.cardId,
        month: months.month,
        year: months.year,
        amountCents: sql<number>`coalesce(sum(${creditCardExpenses.amountCents}), 0)::int`,
      })
      .from(creditCardExpenses)
      .innerJoin(months, eq(creditCardExpenses.monthId, months.id))
      .where(and(eq(creditCardExpenses.userId, userId), isNotNull(creditCardExpenses.installmentId)))
      .groupBy(creditCardExpenses.cardId, months.month, months.year),
    db
      .select({
        amountCents: incomes.amountCents,
        incomeType: incomes.incomeType,
        month: months.month,
        year: months.year,
      })
      .from(incomes)
      .innerJoin(months, eq(incomes.monthId, months.id))
      .where(eq(incomes.userId, userId)),
  ]);

  return {
    cards: cardRows,
    invoices: invoiceRows.map((row) => ({
      ...row,
      status: derivePayableStatus(row.status, row.dueDate),
    })),
    installments: installmentRows.map((row) => ({ ...row, amountCents: Number(row.amountCents) })),
    incomes: incomeRows,
  };
}

/** As faturas de cada cartão num mês, reais ou estimadas. */
export function getCardLinesForMonth(inputs: ForecastInputs, target: MonthParts) {
  return forecastCardMonth(inputs, target, getCurrentMonthParts());
}

/** A nota fiscal do mês, lançada ou estimada. */
export function getMainIncomeForMonth(inputs: ForecastInputs, target: MonthParts) {
  return forecastMainIncome(inputs.incomes, target, getCurrentMonthParts());
}

/**
 * Os próximos `count` meses a partir de `from`, cada um com o que foi lançado
 * e o que ainda é estimado. Mês sem linha em `months` também entra: a fatura
 * de dezembro existe antes de alguém abrir dezembro.
 */
export async function getProjectionForUser(
  userId: string,
  inputs: ForecastInputs,
  from: MonthParts,
  count = 6,
) {
  const totals = await getMonthTotalsForUser(userId);
  const byKey = new Map(totals.map((row) => [`${row.year}-${row.month}`, row]));

  const rows: MonthTotals[] = Array.from({ length: count }, (_, offset) => {
    const parts = getMonthPartsAtOffset(from.month, from.year, offset);
    const known = byKey.get(`${parts.year}-${parts.month}`) ?? {
      ...parts,
      incomeCents: 0,
      billsCents: 0,
      invoicesCents: 0,
    };
    const estimatedInvoicesCents = getCardLinesForMonth(inputs, parts)
      .filter((line) => line.source === "estimated")
      .reduce((sum, line) => sum + line.amountCents, 0);
    const income = getMainIncomeForMonth(inputs, parts);

    return {
      ...known,
      estimatedInvoicesCents,
      estimatedIncomeCents: income.source === "estimated" ? income.amountCents : 0,
    };
  });

  return buildMonthProjection(rows, from, count);
}
