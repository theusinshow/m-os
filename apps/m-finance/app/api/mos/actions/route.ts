import { env } from "@/lib/env";
import { getWhatsappOwnerUser } from "@/lib/whatsapp/auth";
import { createBillFromMosAction } from "@/lib/mos/action-bridge";

// Node.js runtime: Drizzle/pg precisam dele, nao do edge runtime.
export const runtime = "nodejs";

const KNOWN_ACTIONS = new Set(["m-finance.create_bill"]);

/**
 * Executa UMA acao ja proposta pelo Hermes e confirmada no M/OS.
 *
 * O modelo nunca chega aqui direto — quem chama e sempre o M/OS, depois que o
 * usuario confirmou o preview. Autenticacao por secret compartilhado, mesmo
 * padrao do cron do Vercel (`app/api/cron/reminders`).
 */
export async function POST(request: Request) {
  const auth = request.headers.get("authorization");
  const authorized = Boolean(env.mosActionSecret) && auth === `Bearer ${env.mosActionSecret}`;

  if (!authorized) {
    return Response.json({ ok: false, error: "Unauthorized" }, { status: 401 });
  }

  const body = await request.json().catch(() => null);
  const actionId = typeof body?.actionId === "string" ? body.actionId : "";

  if (!KNOWN_ACTIONS.has(actionId)) {
    return Response.json({ ok: false, error: `Ação desconhecida: ${actionId}` }, { status: 400 });
  }

  const owner = await getWhatsappOwnerUser();
  if (!owner) {
    return Response.json({ ok: false, error: "Usuário autorizado não configurado." }, { status: 500 });
  }

  if (actionId === "m-finance.create_bill") {
    const result = await createBillFromMosAction(owner.id, body?.args);
    return Response.json(result, { status: result.ok ? 200 : 422 });
  }

  return Response.json({ ok: false, error: "Ação sem execução implementada." }, { status: 400 });
}
