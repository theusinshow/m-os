import { and, asc, eq, sql } from "drizzle-orm";
import { db } from "@/db/client";
import { billCategories, bills, budgets, creditCardExpenses, creditCardInvoices } from "@/db/schema";
import type { BudgetType } from "@/db/schema";

export type Budget = {
  id: string;
  budgetType: BudgetType;
  categoryId: string | null;
  cardId: string | null;
  limitCents: number;
  spentCents: number;
  remainingCents: number;
  percentage: number;
  label: string;
  isOverBudget: boolean;
  isWarning: boolean;
};

export async function getBudgetsByMonth(monthId: string, userId: string): Promise<Budget[]> {
  if (!db) return [];

  const rows = await db
    .select()
    .from(budgets)
    .where(and(eq(budgets.userId, userId), eq(budgets.monthId, monthId)))
    .orderBy(asc(budgets.budgetType));

  return Promise.all(
    rows.map(async (row) => {
      const spent = await getSpentForBudget(userId, monthId, row.budgetType, row.categoryId, row.cardId);
      const label = await getBudgetLabel(row.budgetType, row.categoryId, row.cardId, userId);
      const remaining = row.limitCents - spent;
      const percentage = row.limitCents > 0 ? Math.round((spent / row.limitCents) * 100) : 0;

      return {
        id: row.id,
        budgetType: row.budgetType,
        categoryId: row.categoryId,
        cardId: row.cardId,
        limitCents: row.limitCents,
        spentCents: spent,
        remainingCents: remaining,
        percentage,
        label,
        isOverBudget: spent > row.limitCents,
        isWarning: percentage >= 80 && !isOverBudget(spent, row.limitCents),
      };
    }),
  );
}

function isOverBudget(spent: number, limit: number) {
  return spent > limit;
}

async function getSpentForBudget(
  userId: string,
  monthId: string,
  type: BudgetType,
  categoryId: string | null,
  cardId: string | null,
): Promise<number> {
  if (!db) return 0;

  // `coalesce(sum(...), 0)::int` é o mesmo padrão de `lib/invoice-sync.ts`:
  // sem o coalesce, um mês sem lançamento devolve `null` em vez de zero, e
  // sem o cast o driver entrega a soma como string.
  //
  // Antes daqui não havia agregação nenhuma: a projeção era a coluna crua e o
  // `[row]` pegava a PRIMEIRA linha. Um orçamento com cinco contas mostrava o
  // valor de uma, e `percentage`, `isOverBudget` e o card "Gasto" desciam
  // todos desse número.
  const billsTotal = sql<number>`coalesce(sum(${bills.amountCents}), 0)::int`;

  if (type === "total") {
    const [billRow] = await db
      .select({ total: billsTotal })
      .from(bills)
      .where(and(eq(bills.userId, userId), eq(bills.monthId, monthId)));
    const [invoiceRow] = await db
      .select({
        total: sql<number>`coalesce(sum(${creditCardInvoices.amountCents}), 0)::int`,
      })
      .from(creditCardInvoices)
      .where(and(eq(creditCardInvoices.userId, userId), eq(creditCardInvoices.monthId, monthId)));
    return (billRow?.total ?? 0) + (invoiceRow?.total ?? 0);
  }

  if (type === "category" && categoryId) {
    const [row] = await db
      .select({ total: billsTotal })
      .from(bills)
      .where(
        and(
          eq(bills.userId, userId),
          eq(bills.monthId, monthId),
          eq(bills.categoryId, categoryId),
        ),
      );
    return row?.total ?? 0;
  }

  if (type === "card" && cardId) {
    const [row] = await db
      .select({
        total: sql<number>`coalesce(sum(${creditCardExpenses.amountCents}), 0)::int`,
      })
      .from(creditCardExpenses)
      .where(
        and(
          eq(creditCardExpenses.userId, userId),
          eq(creditCardExpenses.cardId, cardId),
          eq(creditCardExpenses.monthId, monthId),
        ),
      );
    return row?.total ?? 0;
  }

  return 0;
}

async function getBudgetLabel(
  type: BudgetType,
  categoryId: string | null,
  cardId: string | null,
  userId: string,
): Promise<string> {
  if (type === "total") return "Gasto total do mês";
  if (type === "category" && categoryId) {
    if (!db) return "Categoria";
    const [row] = await db
      .select({ name: billCategories.name })
      .from(billCategories)
      .where(and(eq(billCategories.userId, userId), eq(billCategories.id, categoryId)))
      .limit(1);
    return row?.name ?? "Categoria";
  }
  if (type === "card" && cardId) {
    const { getCardById } = await import("@/lib/card-expenses");
    const card = await getCardById(userId, cardId);
    return card?.name ?? "Cartão";
  }
  return "Orçamento";
}

/**
 * Os lançamentos do orçamento com data, para o acumulado dia a dia.
 *
 * Separado de `getBudgetsByMonth` de propósito: o card precisa de um número e
 * a página inteira o chama para cada orçamento. Trazer a lista de lançamentos
 * junto encareceria todo mundo por causa de um gráfico.
 */
export async function getBudgetEntries(
  userId: string,
  monthId: string,
  type: BudgetType,
  categoryId: string | null,
  cardId: string | null,
): Promise<{ dueDate: string; amountCents: number }[]> {
  if (!db) return [];

  if (type === "card" && cardId) {
    const rows = await db
      .select({
        dueDate: creditCardExpenses.purchaseDate,
        amountCents: creditCardExpenses.amountCents,
      })
      .from(creditCardExpenses)
      .where(
        and(
          eq(creditCardExpenses.userId, userId),
          eq(creditCardExpenses.cardId, cardId),
          eq(creditCardExpenses.monthId, monthId),
        ),
      );

    // `purchase_date` é nullable no schema. Gasto sem data não tem onde cair na
    // linha do tempo, então fica de fora do acumulado — ele continua contando
    // no total que o card já mostra.
    return rows.flatMap((row) =>
      row.dueDate ? [{ dueDate: row.dueDate, amountCents: row.amountCents }] : [],
    );
  }

  const conditions = [eq(bills.userId, userId), eq(bills.monthId, monthId)];
  if (type === "category" && categoryId) {
    conditions.push(eq(bills.categoryId, categoryId));
  }

  const billRows = await db
    .select({ dueDate: bills.dueDate, amountCents: bills.amountCents })
    .from(bills)
    .where(and(...conditions));

  if (type !== "total") return billRows;

  const invoiceRows = await db
    .select({
      dueDate: creditCardInvoices.dueDate,
      amountCents: creditCardInvoices.amountCents,
    })
    .from(creditCardInvoices)
    .where(and(eq(creditCardInvoices.userId, userId), eq(creditCardInvoices.monthId, monthId)));

  return [...billRows, ...invoiceRows];
}
