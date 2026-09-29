import { executeMosAction, isMosActionId } from "@/lib/mos/action-catalog";
import { mosScopeOf } from "@/lib/mos/auth";
import { claimReceipt, completeReceipt, releaseReceipt } from "@/lib/mos/receipts";
import { getWhatsappOwnerUser } from "@/lib/whatsapp/auth";

// Node.js runtime: Drizzle/pg precisam dele, nao do edge runtime.
export const runtime = "nodejs";

const IDEMPOTENCY_KEY = /^[A-Za-z0-9:._-]{8,128}$/;

/**
 * Executa UMA acao ja proposta pelo Hermes e confirmada no M/OS.
 *
 * O modelo nunca chega aqui direto — quem chama e sempre o M/OS, depois que o
 * usuario confirmou o preview. So o secret de ACAO entra: o de leitura
 * (`MOS_FINANCE_READ_SECRET`) nunca escreve (ADR-073).
 *
 * `idempotencyKey` (opcional, recomendado): um retry com a mesma chave devolve
 * o recibo gravado em vez de escrever de novo.
 */
export async function POST(request: Request) {
  if (mosScopeOf(request) !== "action") {
    return Response.json({ ok: false, error: "Unauthorized" }, { status: 401 });
  }

  const body = await request.json().catch(() => null);
  const actionId = typeof body?.actionId === "string" ? body.actionId : "";

  if (!isMosActionId(actionId)) {
    return Response.json({ ok: false, error: `Ação desconhecida: ${actionId.slice(0, 80)}` }, { status: 400 });
  }

  const key = typeof body?.idempotencyKey === "string" ? body.idempotencyKey : null;
  if (key !== null && !IDEMPOTENCY_KEY.test(key)) {
    return Response.json({ ok: false, error: "idempotencyKey inválida." }, { status: 400 });
  }

  const owner = await getWhatsappOwnerUser();
  if (!owner) {
    return Response.json({ ok: false, error: "Usuário autorizado não configurado." }, { status: 500 });
  }

  if (key) {
    const claim = await claimReceipt(owner.id, key, actionId);
    if (claim.status === "done") {
      const stored = claim.result as { ok?: boolean } | null;
      return Response.json({ ...(stored ?? {}), replayed: true }, { status: stored?.ok ? 200 : 422 });
    }
    if (claim.status === "in_progress") {
      return Response.json({ ok: false, error: "Esta ação já está sendo executada." }, { status: 409 });
    }
  }

  try {
    const result = await executeMosAction(actionId, owner.id, body?.args);
    // Recusa de negócio também é desfecho: repetir o pedido daria a mesma recusa.
    if (key) await completeReceipt(owner.id, key, result);
    return Response.json(result, { status: result.ok ? 200 : 422 });
  } catch (error) {
    if (key) await releaseReceipt(owner.id, key);
    console.error(`[mos/actions] ${actionId} falhou: ${error instanceof Error ? error.name : "erro"}`);
    return Response.json({ ok: false, error: "Falha ao executar a ação no M-Finance." }, { status: 500 });
  }
}
