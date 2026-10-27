import { and, desc, eq } from "drizzle-orm";
import { db } from "@/db/client";
import {
  billCategories,
  creditCardExpenses,
  creditCardInvoices,
  months,
  subscriptions,
} from "@/db/schema";
import { composeMonthDate } from "@/lib/due-date";
import { fail, ok, type DomainResult } from "@/lib/domain/finance-actions/result";
import { sumCardExpenses, syncInvoiceTotal } from "@/lib/invoice-sync";
import { ensureConsecutiveMonthsForUser, getMonthPartsAtOffset } from "@/lib/months";
import {
  IMPORT_CATEGORIES,
  learnFrom,
  merchantKey,
  suggestFor,
} from "@/lib/statement-import/classify";
import { parseStatement, type ParsedStatement } from "@/lib/statement-import/parse";
import { isAlreadyImported, planImport, type ReviewedItem } from "@/lib/statement-import/plan";

type Card = { id: string; name: string; dueDay: number };
type MonthParts = { month: number; year: number };

export type PreviewItem = ReviewedItem & {
  suggestionSource: "learned" | "rule" | "none";
  duplicate: boolean;
};

export type StatementPreview = {
  format: ParsedStatement["format"];
  items: PreviewItem[];
  categories: { id: string; name: string; slug: string }[];
  /** O mês em que a fatura vence — onde as compras entram. */
  month: MonthParts;
  owedCents: number;
  creditsCents: number;
  ignored: ParsedStatement["ignored"];
};

/** As categorias do usuário, criando as que a importação usa e ainda não existem. */
async function ensureCategories(userId: string) {
  if (!db) return [];
  await db
    .insert(billCategories)
    .values(IMPORT_CATEGORIES.map((category) => ({ userId, ...category, isDefault: true })))
    .onConflictDoNothing();
  return db
    .select({ id: billCategories.id, name: billCategories.name, slug: billCategories.slug })
    .from(billCategories)
    .where(and(eq(billCategories.userId, userId), eq(billCategories.isArchived, false)))
    .orderBy(billCategories.name);
}

/** O que o usuário já escolheu para cada origem, das compras mais recentes. */
async function learnedChoices(userId: string) {
  if (!db) return new Map();
  const rows = await db
    .select({
      description: creditCardExpenses.description,
      categorySlug: billCategories.slug,
      subscriptionId: creditCardExpenses.subscriptionId,
    })
    .from(creditCardExpenses)
    .leftJoin(billCategories, eq(creditCardExpenses.categoryId, billCategories.id))
    .where(eq(creditCardExpenses.userId, userId))
    .orderBy(desc(creditCardExpenses.updatedAt))
    .limit(1000);
  return learnFrom(
    rows.map((row) => ({
      description: row.description,
      categorySlug: row.categorySlug,
      isSubscription: row.subscriptionId !== null,
    })),
  );
}

/**
 * O mês em que a fatura vence: o primeiro vencimento do cartão depois da
 * compra mais recente. Uma fatura com compras até 26/10 e vencimento dia 10 é
 * a de novembro.
 */
export function statementMonth(parsed: ParsedStatement, dueDay: number, fallback: MonthParts): MonthParts {
  if (parsed.dueDate) {
    const [year, month] = parsed.dueDate.split("-").map(Number);
    return { month, year };
  }
  const latest = parsed.lines
    .map((line) => line.purchaseDate)
    .filter((date): date is string => Boolean(date))
    .sort()
    .at(-1);
  if (!latest) return fallback;
  const [year, month, day] = latest.split("-").map(Number);
  return day < dueDay ? { month, year } : getMonthPartsAtOffset(month, year, 1);
}

async function existingInMonth(userId: string, cardId: string, parts: MonthParts) {
  if (!db) return [];
  return db
    .select({
      externalId: creditCardExpenses.externalId,
      description: creditCardExpenses.description,
      amountCents: creditCardExpenses.amountCents,
      installmentNumber: creditCardExpenses.installmentNumber,
      installmentTotal: creditCardExpenses.installmentTotal,
    })
    .from(creditCardExpenses)
    .innerJoin(months, eq(creditCardExpenses.monthId, months.id))
    .where(
      and(
        eq(creditCardExpenses.userId, userId),
        eq(creditCardExpenses.cardId, cardId),
        eq(months.month, parts.month),
        eq(months.year, parts.year),
      ),
    );
}

export async function previewStatement(
  userId: string,
  card: Card,
  raw: string,
  fallbackMonth: MonthParts,
  monthOverride?: MonthParts,
): Promise<DomainResult<StatementPreview>> {
  if (!db) return fail("db_unavailable", "Banco de dados indisponível no momento.");

  const parsed = parseStatement(raw);
  if (parsed.lines.length === 0) {
    return fail("invalid", "Não encontrei compras nesse conteúdo. Confira se é o .ofx do cartão ou o texto da fatura.");
  }

  const [categories, learned] = await Promise.all([ensureCategories(userId), learnedChoices(userId)]);
  const bySlug = new Map(categories.map((category) => [category.slug, category.id]));
  const month = monthOverride ?? statementMonth(parsed, card.dueDay, fallbackMonth);
  const existing = await existingInMonth(userId, card.id, month);

  const items = parsed.lines.map((line) => {
    const suggestion = suggestFor(line.description, learned);
    const item: ReviewedItem = {
      ...line,
      categoryId: suggestion.categorySlug ? (bySlug.get(suggestion.categorySlug) ?? null) : null,
      isSubscription: suggestion.isSubscription,
    };
    return {
      ...item,
      suggestionSource: suggestion.source,
      duplicate: isAlreadyImported(item, existing),
    };
  });

  const chargesCents = items.reduce((total, item) => total + item.amountCents, 0);
  return ok({
    format: parsed.format,
    items,
    categories,
    month,
    owedCents: parsed.owedCents ?? Math.max(chargesCents - parsed.creditsCents, 0),
    creditsCents: parsed.creditsCents,
    ignored: parsed.ignored,
  });
}

/**
 * A assinatura de cada compra marcada, criando a que falta. Duas cobranças da
 * mesma origem no mesmo mês (as duas contas do ChatGPT) são duas assinaturas:
 * a primeira vai para a primeira, a segunda para a segunda.
 */
async function resolveSubscriptions(
  tx: Parameters<Parameters<NonNullable<typeof db>["transaction"]>[0]>[0],
  userId: string,
  cardId: string,
  items: ReviewedItem[],
) {
  const linked = await tx
    .select({ id: subscriptions.id, name: subscriptions.name })
    .from(subscriptions)
    .where(and(eq(subscriptions.userId, userId), eq(subscriptions.cardId, cardId)))
    .orderBy(subscriptions.createdAt);

  const pool = new Map<string, string[]>();
  for (const sub of linked) {
    const key = merchantKey(sub.name.replace(/\s*\(\d+\)$/, ""));
    pool.set(key, [...(pool.get(key) ?? []), sub.id]);
  }

  const used = new Map<string, number>();
  const byItem = new Map<number, string>();

  for (const [index, item] of items.entries()) {
    if (!item.isSubscription) continue;
    const key = merchantKey(item.description);
    const position = used.get(key) ?? 0;
    used.set(key, position + 1);
    const nextCharge = item.purchaseDate
      ? addOneMonth(item.purchaseDate)
      : composeMonthDate(new Date().getFullYear(), new Date().getMonth() + 2, 1);

    const known = pool.get(key)?.[position];
    if (known) {
      await tx
        .update(subscriptions)
        .set({ amountCents: item.amountCents, nextChargeDate: nextCharge, status: "active", updatedAt: new Date() })
        .where(eq(subscriptions.id, known));
      byItem.set(index, known);
      continue;
    }

    const [created] = await tx
      .insert(subscriptions)
      .values({
        userId,
        cardId,
        name: position === 0 ? item.description : `${item.description} (${position + 1})`,
        amountCents: item.amountCents,
        nextChargeDate: nextCharge,
        cycle: "monthly",
        status: "active",
      })
      .returning({ id: subscriptions.id });
    pool.set(key, [...(pool.get(key) ?? []), created.id]);
    byItem.set(index, created.id);
  }

  return byItem;
}

function addOneMonth(date: string) {
  const [year, month, day] = date.split("-").map(Number);
  const next = getMonthPartsAtOffset(month, year, 1);
  return composeMonthDate(next.year, next.month, day);
}

export type CommitInput = {
  userId: string;
  card: Card;
  month: MonthParts;
  items: ReviewedItem[];
  /** O que falta pagar desta fatura. */
  owedCents: number;
  creditsCents: number;
};

/**
 * Grava a fatura revisada numa transação: compras (com as parcelas futuras),
 * assinaturas ligadas ao cartão e o total da fatura do mês. O total é o que a
 * fatura diz que falta pagar — ele manda; as compras o explicam.
 */
export async function commitStatement(input: CommitInput) {
  if (!db) return fail("db_unavailable", "Banco de dados indisponível no momento.");
  const database = db;

  const existing = await existingInMonth(input.userId, input.card.id, input.month);
  const { rows, skipped } = planImport(input.items, existing);
  const span = Math.max(1, ...rows.map((row) => row.monthOffset + 1));
  const targetMonths = await ensureConsecutiveMonthsForUser(
    input.userId,
    input.month.month,
    input.month.year,
    span,
  );
  const [startMonth] = targetMonths;

  const created = await database.transaction(async (tx) => {
    const previousSums = new Map<string, number>();
    for (const month of targetMonths.slice(1)) {
      previousSums.set(month.id, await sumCardExpenses(tx, input.userId, input.card.id, month.id));
    }

    const skippedIds = new Set(skipped.map((item) => item.externalId));
    const subscriptionByItem = await resolveSubscriptions(
      tx,
      input.userId,
      input.card.id,
      input.items.map((item) =>
        skippedIds.has(item.externalId) ? { ...item, isSubscription: false } : item,
      ),
    );
    const seriesByItem = new Map<number, string>();

    const values = rows.map((row) => {
      let installmentId: string | null = null;
      if (row.installmentNumber !== null) {
        installmentId = seriesByItem.get(row.itemIndex) ?? crypto.randomUUID();
        seriesByItem.set(row.itemIndex, installmentId);
      }
      return {
        userId: input.userId,
        cardId: input.card.id,
        monthId: targetMonths[row.monthOffset].id,
        description: row.description,
        amountCents: row.amountCents,
        purchaseDate: row.purchaseDate,
        categoryId: row.categoryId,
        subscriptionId: row.monthOffset === 0 ? (subscriptionByItem.get(row.itemIndex) ?? null) : null,
        source: "statement" as const,
        externalId: row.externalId,
        installmentId,
        installmentNumber: row.installmentNumber,
        installmentTotal: installmentId ? row.installmentTotal : null,
      };
    });

    if (values.length > 0) {
      await tx.insert(creditCardExpenses).values(values);
    }

    // Uma compra já importada pode ter sido reclassificada na revisão.
    for (const item of skipped) {
      await tx
        .update(creditCardExpenses)
        .set({ categoryId: item.categoryId, updatedAt: new Date() })
        .where(
          and(
            eq(creditCardExpenses.userId, input.userId),
            eq(creditCardExpenses.externalId, item.externalId),
          ),
        );
    }

    if (input.owedCents > 0) {
      await tx
        .insert(creditCardInvoices)
        .values({
          userId: input.userId,
          cardId: input.card.id,
          monthId: startMonth.id,
          amountCents: input.owedCents,
          creditsCents: input.creditsCents,
          dueDate: composeMonthDate(startMonth.year, startMonth.month, input.card.dueDay),
          status: "pending",
        })
        .onConflictDoUpdate({
          target: [creditCardInvoices.cardId, creditCardInvoices.monthId],
          set: { amountCents: input.owedCents, creditsCents: input.creditsCents, updatedAt: new Date() },
        });
    }

    // Os meses seguintes ganharam parcelas: a previsão da fatura acompanha.
    for (const month of targetMonths.slice(1)) {
      await syncInvoiceTotal(
        tx,
        input.userId,
        input.card.id,
        month,
        input.card.dueDay,
        previousSums.get(month.id) ?? 0,
      );
    }

    return values.length;
  });

  return ok({ created, skipped: skipped.length, months: targetMonths.length });
}
