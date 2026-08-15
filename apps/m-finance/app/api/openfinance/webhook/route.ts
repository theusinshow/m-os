import { eq } from "drizzle-orm";
import { db } from "@/db/client";
import { pluggyItems } from "@/db/schema";
import { env } from "@/lib/env";
import { syncPluggyItem } from "@/lib/openfinance/sync";

// Events that mean "there may be new transactions to import".
const SYNC_EVENTS = new Set([
  "item/created",
  "item/updated",
  "transactions/created",
  "transactions/updated",
]);

/**
 * Pluggy webhook. Configure its URL in the Pluggy dashboard as
 * `https://<host>/api/openfinance/webhook?secret=<PLUGGY_WEBHOOK_SECRET>`.
 *
 * It's unauthenticated (no Supabase session), so we (a) require the shared
 * secret and (b) resolve the owning user from the stored item mapping rather
 * than trusting anything in the payload.
 */
export async function POST(request: Request) {
  const secret = new URL(request.url).searchParams.get("secret");
  if (!env.pluggyWebhookSecret || secret !== env.pluggyWebhookSecret) {
    return new Response("Unauthorized", { status: 401 });
  }

  if (!db) {
    return new Response("DB unavailable", { status: 503 });
  }

  let event: string | undefined;
  let itemId: string | undefined;
  try {
    const body = (await request.json()) as { event?: string; itemId?: string };
    event = body.event;
    itemId = body.itemId;
  } catch {
    return new Response("Invalid payload", { status: 400 });
  }

  // Always 200 on accepted-but-ignored events so Pluggy doesn't retry forever.
  if (!itemId || !event || !SYNC_EVENTS.has(event)) {
    return Response.json({ ignored: true });
  }

  const [mapping] = await db
    .select({ userId: pluggyItems.userId })
    .from(pluggyItems)
    .where(eq(pluggyItems.itemId, itemId))
    .limit(1);

  if (!mapping) {
    return Response.json({ ignored: true, reason: "unknown item" });
  }

  try {
    const results = await syncPluggyItem(mapping.userId, itemId);
    return Response.json({ ok: true, results });
  } catch (error) {
    const message = error instanceof Error ? error.message : "sync failed";
    return Response.json({ ok: false, error: message }, { status: 502 });
  }
}
