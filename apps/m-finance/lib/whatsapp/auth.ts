import { eq } from "drizzle-orm";
import { db } from "@/db/client";
import { users } from "@/db/schema";
import { env } from "@/lib/env";

export function normalizeWhatsappPhone(value: string | null | undefined) {
  return (value ?? "").trim();
}

export function isAllowedWhatsappSender(from: string | null | undefined) {
  const configuredPhone = normalizeWhatsappPhone(env.whatsappAllowedPhone);

  if (!configuredPhone) {
    return false;
  }

  return normalizeWhatsappPhone(from) === configuredPhone;
}

export function isAuthorizedWhatsappWebhook(request: Request) {
  if (!env.whatsappWebhookSecret) {
    return false;
  }

  const secret = new URL(request.url).searchParams.get("secret");
  return secret === env.whatsappWebhookSecret;
}

export async function getWhatsappOwnerUser() {
  if (!db || !env.authorizedEmail) {
    return null;
  }

  const [user] = await db
    .select()
    .from(users)
    .where(eq(users.email, env.authorizedEmail))
    .limit(1);

  return user ?? null;
}
