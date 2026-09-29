import { env } from "@/lib/env";
import { runFinancialObserver } from "@/lib/finance-intelligence/observer/run";
import { getWhatsappOwnerUser } from "@/lib/whatsapp/auth";
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
 * vencimentos do dia (sempre) e resumo semanal (às segundas) — e o Financial
 * Observer (ADR-073), que grava os insights materiais do dia.
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

  const [push, whatsappDue, whatsappWeekly, observer] = await Promise.all([
    runSubscriptionReminders(),
    runWhatsappDueReminders(),
    runWhatsappWeeklySummary(),
    runObserverForOwner(),
  ]);

  return Response.json({
    ok: true,
    push,
    whatsapp: { due: whatsappDue, weekly: whatsappWeekly },
    observer,
  });
}

/** O Observer não derruba o cron: lembrete de assinatura é mais antigo e mais crítico. */
async function runObserverForOwner() {
  try {
    const owner = await getWhatsappOwnerUser();
    if (!owner) return { skipped: "owner_not_configured" };
    return await runFinancialObserver(owner.id);
  } catch (error) {
    console.error(`[cron/observer] falhou: ${error instanceof Error ? error.name : "erro"}`);
    return { skipped: "error" };
  }
}
