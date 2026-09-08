import { and, desc, eq, sql } from "drizzle-orm";
import { db } from "@/db/client";
import { bills, creditCardInvoices, incomes, months, users } from "@/db/schema";

export function getCurrentMonthParts(date = new Date()) {
  return {
    month: date.getMonth() + 1,
    year: date.getFullYear(),
  };
}

export function getNextMonthParts(date = new Date()) {
  const next = new Date(date.getFullYear(), date.getMonth() + 1, 1);

  return {
    month: next.getMonth() + 1,
    year: next.getFullYear(),
  };
}

export function getMonthPartsAtOffset(month: number, year: number, offset: number) {
  const target = new Date(year, month - 1 + offset, 1);

  return {
    month: target.getMonth() + 1,
    year: target.getFullYear(),
  };
}

export async function getAppUserBySupabaseId(supabaseUserId: string) {
  if (!db) {
    return null;
  }

  const [appUser] = await db
    .select()
    .from(users)
    .where(eq(users.supabaseUserId, supabaseUserId))
    .limit(1);

  return appUser ?? null;
}

export async function getCurrentMonthForUser(userId: string) {
  if (!db) {
    return null;
  }

  const current = getCurrentMonthParts();
  const [month] = await db
    .select()
    .from(months)
    .where(and(eq(months.userId, userId), eq(months.month, current.month), eq(months.year, current.year)))
    .limit(1);

  return month ?? null;
}

export async function createCurrentMonthForUser(userId: string) {
  if (!db) {
    throw new Error("Database is not configured.");
  }

  const current = getCurrentMonthParts();
  const [month] = await db
    .insert(months)
    .values({
      userId,
      month: current.month,
      year: current.year,
    })
    .onConflictDoUpdate({
      target: [months.userId, months.month, months.year],
      set: {
        updatedAt: new Date(),
      },
    })
    .returning();

  return month;
}

/**
 * Returns the month row for the given parts, creating it if missing. Imported
 * transactions can land in months the user never opened manually, so the sync
 * needs to materialize them on demand.
 */
export async function ensureMonthForUser(userId: string, month: number, year: number) {
  if (!db) {
    throw new Error("Database is not configured.");
  }

  const [row] = await db
    .insert(months)
    .values({ userId, month, year })
    .onConflictDoUpdate({
      target: [months.userId, months.month, months.year],
      set: { updatedAt: new Date() },
    })
    .returning();

  return row;
}

export async function ensureConsecutiveMonthsForUser(
  userId: string,
  startMonth: number,
  startYear: number,
  count: number,
) {
  if (!db) {
    throw new Error("Database is not configured.");
  }

  const parts = Array.from({ length: count }, (_, index) =>
    getMonthPartsAtOffset(startMonth, startYear, index),
  );

  const rows = await db
    .insert(months)
    .values(parts.map((part) => ({ userId, ...part })))
    .onConflictDoUpdate({
      target: [months.userId, months.month, months.year],
      set: { updatedAt: new Date() },
    })
    .returning();

  const byKey = new Map(rows.map((row) => [`${row.year}-${row.month}`, row]));
  return parts.map((part) => {
    const row = byKey.get(`${part.year}-${part.month}`);
    if (!row) {
      throw new Error("Não foi possível preparar os meses da série.");
    }
    return row;
  });
}

export async function getMonthByParts(userId: string, month: number, year: number) {
  if (!db) {
    return null;
  }

  const [row] = await db
    .select()
    .from(months)
    .where(and(eq(months.userId, userId), eq(months.month, month), eq(months.year, year)))
    .limit(1);

  return row ?? null;
}

export async function getMonthsForUser(userId: string) {
  if (!db) {
    return [];
  }

  return db
    .select()
    .from(months)
    .where(eq(months.userId, userId))
    .orderBy(desc(months.year), desc(months.month));
}

export async function getNextMonthForUser(userId: string) {
  if (!db) {
    return null;
  }

  const next = getNextMonthParts();
  const [month] = await db
    .select()
    .from(months)
    .where(and(eq(months.userId, userId), eq(months.month, next.month), eq(months.year, next.year)))
    .limit(1);

  return month ?? null;
}

/**
 * Os meses que têm alguma coisa dentro — conta, fatura ou receita.
 *
 * O seletor precisa distinguir "mês que existe" de "mês que aconteceu": uma
 * série parcelada cria a linha de 22 meses de uma vez, e um passado vazio no
 * seletor é só um lugar para o app se perder.
 */
export async function getMonthIdsWithActivity(userId: string) {
  if (!db) {
    return new Set<string>();
  }

  const [billMonths, invoiceMonths, incomeMonths] = await Promise.all([
    db.selectDistinct({ monthId: bills.monthId }).from(bills).where(eq(bills.userId, userId)),
    db
      .selectDistinct({ monthId: creditCardInvoices.monthId })
      .from(creditCardInvoices)
      .where(eq(creditCardInvoices.userId, userId)),
    db.selectDistinct({ monthId: incomes.monthId }).from(incomes).where(eq(incomes.userId, userId)),
  ]);

  return new Set(
    [...billMonths, ...invoiceMonths, ...incomeMonths].map((row) => row.monthId),
  );
}

/**
 * Receita, contas e faturas somadas por mês, para a projeção.
 *
 * Três agregações separadas em vez de um join: somar as três tabelas numa
 * consulta só multiplica linha (um mês com 6 contas e 4 faturas viraria 24
 * linhas) e infla os totais.
 */
export async function getMonthTotalsForUser(userId: string) {
  if (!db) {
    return [];
  }

  const [monthRows, incomeRows, billRows, invoiceRows] = await Promise.all([
    db
      .select({ id: months.id, month: months.month, year: months.year })
      .from(months)
      .where(eq(months.userId, userId)),
    db
      .select({
        monthId: incomes.monthId,
        total: sql<number>`coalesce(sum(${incomes.amountCents}), 0)::int`,
      })
      .from(incomes)
      .where(eq(incomes.userId, userId))
      .groupBy(incomes.monthId),
    db
      .select({
        monthId: bills.monthId,
        total: sql<number>`coalesce(sum(${bills.amountCents}), 0)::int`,
      })
      .from(bills)
      .where(eq(bills.userId, userId))
      .groupBy(bills.monthId),
    db
      .select({
        monthId: creditCardInvoices.monthId,
        total: sql<number>`coalesce(sum(${creditCardInvoices.amountCents}), 0)::int`,
      })
      .from(creditCardInvoices)
      .where(eq(creditCardInvoices.userId, userId))
      .groupBy(creditCardInvoices.monthId),
  ]);

  const byMonth = (rows: { monthId: string; total: number }[]) =>
    new Map(rows.map((row) => [row.monthId, Number(row.total)]));
  const incomeByMonth = byMonth(incomeRows);
  const billByMonth = byMonth(billRows);
  const invoiceByMonth = byMonth(invoiceRows);

  return monthRows.map((row) => ({
    month: row.month,
    year: row.year,
    incomeCents: incomeByMonth.get(row.id) ?? 0,
    billsCents: billByMonth.get(row.id) ?? 0,
    invoicesCents: invoiceByMonth.get(row.id) ?? 0,
  }));
}
