import type { User } from "@supabase/supabase-js";
import { and, eq } from "drizzle-orm";
import { db } from "@/db/client";
import { billCategories, creditCards, settings, users } from "@/db/schema";

const initialCategories = [
  "Moradia",
  "Transporte",
  "Moto",
  "Faculdade",
  "Assinaturas",
  "Saúde",
  "Cartão",
  "Dívidas",
  "Lazer",
  "Investimentos",
  "Outros",
];

const initialCards = [
  { name: "Mercado Pago", cardType: "personal" as const, dueDay: 10 },
  { name: "Itaú", cardType: "personal" as const, dueDay: 10 },
  { name: "Nubank Pessoal", cardType: "personal" as const, dueDay: 10 },
  { name: "Nubank PJ", cardType: "business" as const, dueDay: 10 },
];

function slugify(value: string) {
  return value
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/(^-|-$)/g, "");
}

export async function ensureAppUser(user: User) {
  if (!db || !user.email) {
    return null;
  }

  const [appUser] = await db
    .insert(users)
    .values({
      supabaseUserId: user.id,
      name: user.user_metadata?.name ?? user.email,
      email: user.email,
      avatarUrl: user.user_metadata?.avatar_url ?? null,
    })
    .onConflictDoUpdate({
      target: users.supabaseUserId,
      set: {
        name: user.user_metadata?.name ?? user.email,
        email: user.email,
        avatarUrl: user.user_metadata?.avatar_url ?? null,
        updatedAt: new Date(),
      },
    })
    .returning();

  await db
    .insert(settings)
    .values({ userId: appUser.id })
    .onConflictDoNothing({ target: settings.userId });

  await db
    .insert(billCategories)
    .values(
      initialCategories.map((name) => ({
        userId: appUser.id,
        name,
        slug: slugify(name),
        isDefault: true,
      })),
    )
    .onConflictDoNothing();

  const existingCards = await db
    .select({ cardType: creditCards.cardType, name: creditCards.name })
    .from(creditCards)
    .where(and(eq(creditCards.userId, appUser.id), eq(creditCards.isActive, true)));

  const existingCardKeys = new Set(
    existingCards.map((card) => `${card.name.trim().toLowerCase()}-${card.cardType}`),
  );

  const missingInitialCards = initialCards.filter(
    (card) => !existingCardKeys.has(`${card.name.trim().toLowerCase()}-${card.cardType}`),
  );

  if (missingInitialCards.length > 0) {
    await db.insert(creditCards).values(
      missingInitialCards.map((card) => ({
        userId: appUser.id,
        name: card.name,
        cardType: card.cardType,
        dueDay: card.dueDay,
      })),
    );
  }

  return appUser;
}
