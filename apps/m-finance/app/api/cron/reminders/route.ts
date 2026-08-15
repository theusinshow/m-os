import { env } from "@/lib/env";
import { runSubscriptionReminders } from "@/lib/push/reminders";

// Run on the Node.js runtime (web-push needs Node crypto, not the edge runtime).
export const runtime = "nodejs";

/**
 * Daily cron that sends "you're about to be charged" reminders.
 *
 * Vercel Cron calls this with `Authorization: Bearer <CRON_SECRET>` when the
 * CRON_SECRET env var is set. We also accept `?secret=` for manual testing.
 */
export async function GET(request: Request) {
  const auth = request.headers.get("authorization");
  const querySecret = new URL(request.url).searchParams.get("secret");

  const authorized =
    Boolean(env.cronSecret) &&
    (auth === `Bearer ${env.cronSecret}` || querySecret === env.cronSecret);

  if (!authorized) {
    return new Response("Unauthorized", { status: 401 });
  }

  const result = await runSubscriptionReminders();
  return Response.json({ ok: true, ...result });
}
