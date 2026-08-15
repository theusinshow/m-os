import { and, desc, eq } from "drizzle-orm";
import { db } from "@/db/client";
import { months, users } from "@/db/schema";

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
