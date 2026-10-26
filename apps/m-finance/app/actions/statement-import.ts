"use server";

import { revalidatePath } from "next/cache";
import { z } from "zod";
import { requireUser } from "@/lib/auth/guard";
import { getActiveMonthForUser } from "@/lib/active-month";
import { getCardById } from "@/lib/card-expenses";
import { getAppUserBySupabaseId, getCurrentMonthParts } from "@/lib/months";
import { commitStatement, previewStatement } from "@/lib/statement-import/service";

const monthSchema = z.object({
  month: z.number().int().min(1).max(12),
  year: z.number().int().min(2020),
});

async function resolveCard(cardId: string) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  if (!appUser) return null;
  const card = await getCardById(appUser.id, cardId);
  return card ? { appUser, card } : null;
}

// Um .ofx de fatura tem dezenas de KB; um teto folgado impede colar um livro.
const MAX_STATEMENT_CHARS = 2_000_000;

export async function previewStatementAction(input: {
  cardId: string;
  raw: string;
  month?: { month: number; year: number };
}) {
  const resolved = await resolveCard(input.cardId);
  if (!resolved) return { ok: false as const, message: "Cartão não encontrado." };
  if (!input.raw.trim() || input.raw.length > MAX_STATEMENT_CHARS) {
    return { ok: false as const, message: "Envie o arquivo da fatura ou cole o texto dela." };
  }

  const active = await getActiveMonthForUser(resolved.appUser.id);
  const fallback = active ? { month: active.month, year: active.year } : getCurrentMonthParts();
  const override = input.month ? monthSchema.parse(input.month) : undefined;
  const result = await previewStatement(resolved.appUser.id, resolved.card, input.raw, fallback, override);
  return result.ok
    ? { ok: true as const, preview: result.value }
    : { ok: false as const, message: result.message };
}

const commitSchema = z.object({
  cardId: z.string().uuid(),
  month: monthSchema,
  owedCents: z.number().int().min(0),
  creditsCents: z.number().int().min(0),
  items: z
    .array(
      z.object({
        externalId: z.string().min(1).max(200),
        description: z.string().trim().min(1).max(200),
        amountCents: z.number().int().positive(),
        purchaseDate: z.string().regex(/^\d{4}-\d{2}-\d{2}$/).nullable(),
        installmentNumber: z.number().int().min(1).nullable(),
        installmentTotal: z.number().int().min(2).max(72).nullable(),
        categoryId: z.string().uuid().nullable(),
        isSubscription: z.boolean(),
      }),
    )
    .min(1)
    .max(500),
});

export async function commitStatementAction(input: z.input<typeof commitSchema>) {
  const parsed = commitSchema.safeParse(input);
  if (!parsed.success) {
    return { ok: false as const, message: "Algumas compras estão incompletas. Revise e tente de novo." };
  }

  const resolved = await resolveCard(parsed.data.cardId);
  if (!resolved) return { ok: false as const, message: "Cartão não encontrado." };

  const result = await commitStatement({
    userId: resolved.appUser.id,
    card: resolved.card,
    month: parsed.data.month,
    items: parsed.data.items,
    owedCents: parsed.data.owedCents,
    creditsCents: parsed.data.creditsCents,
  });
  if (!result.ok) return { ok: false as const, message: result.message };

  revalidatePath(`/app/cards/${resolved.card.id}`);
  revalidatePath("/app/cards");
  revalidatePath("/app/dashboard");
  revalidatePath("/app/calendar");
  revalidatePath("/app/subscriptions");
  return { ok: true as const, ...result.value };
}
