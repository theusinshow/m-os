import { env } from "@/lib/env";
import { runSubscriptionReminders } from "@/lib/push/reminders";
import {
  runWhatsappDueReminders,
  runWhatsappWeeklySummary,
} from "@/lib/whatsapp/notifications";

// Run on the Node.js runtime (web-push and Twilio need Node, not the edge runtime).
export const runtime = "nodejs";

/**
 * Daily cron that sends "you're about to be charged" reminders.
 *
 * Vercel Cron calls this with `Authorization: Bearer <CRON_SECRET>` when the
 * CRON_SECRET env var is set. We also accept `?secret=` for manual testing.
 *
 * Além dos lembretes de assinatura (web push), roda as notificações do WhatsApp:
 * vencimentos do dia (sempre) e resumo semanal (às segundas).
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

  const [push, whatsappDue, whatsappWeekly] = await Promise.all([
    runSubscriptionReminders(),
    runWhatsappDueReminders(),
    runWhatsappWeeklySummary(),
  ]);

  return Response.json({
    ok: true,
    push,
    whatsapp: { due: whatsappDue, weekly: whatsappWeekly },
  });
}
