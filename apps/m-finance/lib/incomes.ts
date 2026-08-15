import { eq } from "drizzle-orm";
import { db } from "@/db/client";
import { incomes } from "@/db/schema";

export async function getIncomesByMonth(monthId: string) {
  if (!db) {
    return [];
  }

  return db.select().from(incomes).where(eq(incomes.monthId, monthId));
}
