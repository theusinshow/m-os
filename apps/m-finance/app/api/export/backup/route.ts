import { eq } from "drizzle-orm";
import { db } from "@/db/client";
import {
  alerts,
  billCategories,
  bills,
  creditCardExpenses,
  creditCardInvoices,
  creditCards,
  goalContributions,
  goals,
  incomes,
  monthlySnapshots,
  months,
  purchaseSimulations,
  recurrenceRules,
  settings,
} from "@/db/schema";
import { requireUser } from "@/lib/auth/guard";
import { getAppUserBySupabaseId } from "@/lib/months";

export async function GET() {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  if (!db || !appUser) {
    return new Response("Usuário interno não configurado.", { status: 400 });
  }

  const uid = appUser.id;

  const [
    monthsData,
    incomesData,
    categoriesData,
    recurrenceData,
    billsData,
    cardsData,
    invoicesData,
    expensesData,
    alertsData,
    snapshotsData,
    simulationsData,
    goalsData,
    contributionsData,
    settingsData,
  ] = await Promise.all([
    db.select().from(months).where(eq(months.userId, uid)),
    db.select().from(incomes).where(eq(incomes.userId, uid)),
    db.select().from(billCategories).where(eq(billCategories.userId, uid)),
    db.select().from(recurrenceRules).where(eq(recurrenceRules.userId, uid)),
    db.select().from(bills).where(eq(bills.userId, uid)),
    db.select().from(creditCards).where(eq(creditCards.userId, uid)),
    db.select().from(creditCardInvoices).where(eq(creditCardInvoices.userId, uid)),
    db.select().from(creditCardExpenses).where(eq(creditCardExpenses.userId, uid)),
    db.select().from(alerts).where(eq(alerts.userId, uid)),
    db.select().from(monthlySnapshots).where(eq(monthlySnapshots.userId, uid)),
    db.select().from(purchaseSimulations).where(eq(purchaseSimulations.userId, uid)),
    db.select().from(goals).where(eq(goals.userId, uid)),
    db.select().from(goalContributions).where(eq(goalContributions.userId, uid)),
    db.select().from(settings).where(eq(settings.userId, uid)),
  ]);

  const backup = {
    version: 1,
    exportedFor: appUser.email,
    data: {
      months: monthsData,
      incomes: incomesData,
      billCategories: categoriesData,
      recurrenceRules: recurrenceData,
      bills: billsData,
      creditCards: cardsData,
      creditCardInvoices: invoicesData,
      creditCardExpenses: expensesData,
      alerts: alertsData,
      monthlySnapshots: snapshotsData,
      purchaseSimulations: simulationsData,
      goals: goalsData,
      goalContributions: contributionsData,
      settings: settingsData,
    },
  };

  return new Response(JSON.stringify(backup, null, 2), {
    headers: {
      "content-type": "application/json; charset=utf-8",
      "content-disposition": `attachment; filename="m-finance-backup.json"`,
    },
  });
}
