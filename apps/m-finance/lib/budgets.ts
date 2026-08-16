import { and, asc, eq } from "drizzle-orm";
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

  if (type === "total") {
    const [billRow] = await db
      .select({ total: bills.amountCents })
      .from(bills)
      .where(and(eq(bills.userId, userId), eq(bills.monthId, monthId)));
    const [invoiceRow] = await db
      .select({ total: creditCardInvoices.amountCents })
      .from(creditCardInvoices)
      .where(and(eq(creditCardInvoices.userId, userId), eq(creditCardInvoices.monthId, monthId)));
    return (billRow?.total ?? 0) + (invoiceRow?.total ?? 0);
  }

  if (type === "category" && categoryId) {
    const [row] = await db
      .select({ total: bills.amountCents })
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
      .select({ total: creditCardExpenses.amountCents })
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
