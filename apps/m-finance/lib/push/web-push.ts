import webpush from "web-push";
import { eq } from "drizzle-orm";
import { db } from "@/db/client";
import { pushSubscriptions } from "@/db/schema";
import { env, isPushConfigured } from "@/lib/env";

let configured = false;

function ensureConfigured() {
  if (configured) return;
  webpush.setVapidDetails(env.vapidSubject, env.vapidPublicKey, env.vapidPrivateKey);
  configured = true;
}

export type PushPayload = {
  title: string;
  body: string;
  url?: string;
  tag?: string;
};

/**
 * Sends a push to every device the user registered, pruning any endpoint the
 * push service reports as gone (404/410). Returns how many were delivered.
 */
export async function sendPushToUser(userId: string, payload: PushPayload): Promise<number> {
  if (!db || !isPushConfigured()) return 0;
  ensureConfigured();

  const subs = await db
    .select()
    .from(pushSubscriptions)
    .where(eq(pushSubscriptions.userId, userId));

  let delivered = 0;
  const body = JSON.stringify(payload);

  for (const sub of subs) {
    try {
      await webpush.sendNotification(
        { endpoint: sub.endpoint, keys: { p256dh: sub.p256dh, auth: sub.auth } },
        body,
      );
      delivered += 1;
    } catch (error) {
      const status = (error as { statusCode?: number })?.statusCode;
      // Endpoint is dead — drop it so we stop trying.
      if (status === 404 || status === 410) {
        await db.delete(pushSubscriptions).where(eq(pushSubscriptions.id, sub.id));
      }
    }
  }

  return delivered;
}
