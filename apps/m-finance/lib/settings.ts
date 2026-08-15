import { asc, eq } from "drizzle-orm";
import { db } from "@/db/client";
import { billCategories, settings } from "@/db/schema";

export async function getSettingsForUser(userId: string) {
  if (!db) {
    return null;
  }

  const [row] = await db.select().from(settings).where(eq(settings.userId, userId)).limit(1);

  return row ?? null;
}

export async function getAllCategories(userId: string) {
  if (!db) {
    return [];
  }

  return db
    .select({
      id: billCategories.id,
      name: billCategories.name,
      isArchived: billCategories.isArchived,
    })
    .from(billCategories)
    .where(eq(billCategories.userId, userId))
    .orderBy(asc(billCategories.isArchived), asc(billCategories.name));
}
