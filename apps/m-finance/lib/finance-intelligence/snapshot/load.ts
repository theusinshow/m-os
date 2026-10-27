import { and, eq, gte, lte, or, sql } from "drizzle-orm";
import { db } from "@/db/client";
import {
  billCategories,
  bills,
  creditCardExpenses,
  creditCardInvoices,
  creditCards,
  financialPolicies,
  goals,
  incomes,
  months,
  subscriptions,
} from "@/db/schema";
import { addMonths, monthIndex, monthOfDate, todayInSaoPaulo } from "@/lib/finance-intelligence/dates";
import { resolvePolicies } from "@/lib/finance-intelligence/policies";
import type { FinanceSnapshot } from "@/lib/finance-intelligence/types";

/** Quantos meses para trás e para frente o snapshot carrega. */
export const SNAPSHOT_MONTHS_BACK = 6;
export const SNAPSHOT_MONTHS_FORWARD = 24;

export class FinanceDataUnavailableError extends Error {
  constructor() {
    super("Banco de dados indisponível.");
  }
}

/**
 * A única leitura de banco do kernel.
 *
 * Tudo em paralelo e numa janela de meses fixa (−6/+24): para uso pessoal são
 * dezenas a poucas centenas de linhas, e ler uma vez só é o que garante que as
 * ferramentas de um mesmo pedido falam do mesmo instante. Para trás, a janela
 * cobre as três faturas que a estimativa usa com folga; para frente, o
 * horizonte máximo de cenário (24 meses). Parcela além disso não muda nenhuma
 * resposta que o kernel dá.
 */
export async function loadFinanceSnapshot(userId: string, now = new Date()): Promise<FinanceSnapshot> {
  if (!db) throw new FinanceDataUnavailableError();

  const today = todayInSaoPaulo(now);
  const current = monthOfDate(today);
  const from = addMonths(current, -SNAPSHOT_MONTHS_BACK);
  const until = addMonths(current, SNAPSHOT_MONTHS_FORWARD);
  const fromIndex = monthIndex(from);
  const untilIndex = monthIndex(until);
  // months.year * 12 + (months.month - 1) — o mesmo índice de `dates.ts`.
  const monthWindow = and(
    gte(sql`${months.year} * 12 + (${months.month} - 1)`, fromIndex),
    lte(sql`${months.year} * 12 + (${months.month} - 1)`, untilIndex),
  );

  const [incomeRows, billRows, cardRows, invoiceRows, expenseRows, subscriptionRows, goalRows, policyRows] =
    await Promise.all([
      db
        .select({
          id: incomes.id,
          name: incomes.name,
          amountCents: incomes.amountCents,
          incomeType: incomes.incomeType,
          expectedDate: incomes.expectedDate,
          received: incomes.received,
          month: months.month,
          year: months.year,
        })
        .from(incomes)
        .innerJoin(months, eq(incomes.monthId, months.id))
        .where(and(eq(incomes.userId, userId), monthWindow)),
      db
        .select({
          id: bills.id,
          name: bills.name,
          amountCents: bills.amountCents,
          dueDate: bills.dueDate,
          status: bills.status,
          isRecurring: bills.isRecurring,
          recurrenceRuleId: bills.recurrenceRuleId,
          seriesId: bills.seriesId,
          seriesNumber: bills.seriesNumber,
          seriesTotal: bills.seriesTotal,
          categoryName: billCategories.name,
          month: months.month,
          year: months.year,
        })
        .from(bills)
        .innerJoin(months, eq(bills.monthId, months.id))
        .leftJoin(billCategories, eq(bills.categoryId, billCategories.id))
        .where(and(eq(bills.userId, userId), monthWindow)),
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
        .where(and(eq(creditCardInvoices.userId, userId), monthWindow)),
      db
        .select({
          id: creditCardExpenses.id,
          cardId: creditCardExpenses.cardId,
          description: creditCardExpenses.description,
          amountCents: creditCardExpenses.amountCents,
          purchaseDate: creditCardExpenses.purchaseDate,
          installmentId: creditCardExpenses.installmentId,
          installmentNumber: creditCardExpenses.installmentNumber,
          installmentTotal: creditCardExpenses.installmentTotal,
          month: months.month,
          year: months.year,
        })
        .from(creditCardExpenses)
        .innerJoin(months, eq(creditCardExpenses.monthId, months.id))
        .where(and(eq(creditCardExpenses.userId, userId), monthWindow)),
      db
        .select({
          id: subscriptions.id,
          name: subscriptions.name,
          amountCents: subscriptions.amountCents,
          nextChargeDate: subscriptions.nextChargeDate,
          cycle: subscriptions.cycle,
          status: subscriptions.status,
        })
        .from(subscriptions)
        .where(eq(subscriptions.userId, userId)),
      db
        .select({
          id: goals.id,
          name: goals.name,
          targetAmountCents: goals.targetAmountCents,
          currentAmountCents: goals.currentAmountCents,
          deadline: goals.deadline,
          priority: goals.priority,
          status: goals.status,
        })
        .from(goals)
        .where(and(eq(goals.userId, userId), or(eq(goals.status, "active"), eq(goals.status, "paused"), eq(goals.status, "completed")))),
      // Sem a migration 0016 aplicada a tabela não existe: o kernel segue com
      // as políticas padrão em vez de derrubar o dashboard e o Hermes juntos.
      db
        .select({ key: financialPolicies.key, value: financialPolicies.value, active: financialPolicies.active })
        .from(financialPolicies)
        .where(eq(financialPolicies.userId, userId))
        .catch(() => []),
    ]);

  return {
    today,
    current,
    incomes: incomeRows,
    bills: billRows,
    cards: cardRows,
    invoices: invoiceRows,
    cardExpenses: expenseRows,
    subscriptions: subscriptionRows,
    goals: goalRows,
    policies: resolvePolicies(policyRows),
  };
}
