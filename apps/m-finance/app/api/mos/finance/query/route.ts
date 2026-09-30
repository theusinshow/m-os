import { FinanceAiNotConfiguredError } from "@/lib/finance-intelligence/ai/provider";
import { FINANCE_TOOLS, isFinanceToolId, type ToolContext } from "@/lib/finance-intelligence/gateway/tools";
import { liveInsightsFor } from "@/lib/finance-intelligence/observer/live";
import { getPreviousSafeToSpend, getStoredInsights } from "@/lib/finance-intelligence/observer/run";
import { FinanceDataUnavailableError, loadFinanceSnapshot } from "@/lib/finance-intelligence/snapshot/load";
import type { FinanceSnapshot } from "@/lib/finance-intelligence/types";
import { mosScopeOf } from "@/lib/mos/auth";
import { allowRequest } from "@/lib/mos/rate-limit";
import { getWhatsappOwnerUser } from "@/lib/whatsapp/auth";

// Drizzle/postgres precisam do runtime Node. A análise pesada pode demorar.
export const runtime = "nodejs";
export const maxDuration = 60;

const MAX_BODY_BYTES = 16 * 1024;
const MAX_RESPONSE_BYTES = 256 * 1024;
// Charset explicito: o PowerShell 5.1 le JSON sem charset como Latin-1 e
// transforma "Pressão" em "PressÃ£o".
const JSON_UTF8 = { "content-type": "application/json; charset=utf-8" };

function failure(status: number, tool: string, code: string, message: string) {
  return Response.json({ ok: false, tool, error: { code, message } }, { status, headers: JSON_UTF8 });
}

/**
 * Intelligence Gateway — a leitura do M-Finance pelo M/OS (ADR-073).
 *
 * `{ tool, args }` → `{ ok, tool, asOf, data }`. Só leitura: as ferramentas
 * recebem um snapshot já carregado, e nenhuma delas recebe o banco. Quem chama
 * é sempre o M/OS, com o secret guardado no Credential Manager; o modelo nunca
 * chega aqui direto.
 */
export async function POST(request: Request) {
  const scope = mosScopeOf(request);
  if (!scope) return failure(401, "", "unauthorized", "Unauthorized");

  if (!allowRequest(`finance-query:${scope}`)) {
    return failure(429, "", "rate_limited", "Muitas consultas em pouco tempo. Tente de novo em um minuto.");
  }

  const raw = await request.text().catch(() => "");
  if (Buffer.byteLength(raw) > MAX_BODY_BYTES) {
    return failure(413, "", "payload_too_large", "Pedido grande demais.");
  }

  let body: { tool?: unknown; args?: unknown } | null = null;
  try {
    body = JSON.parse(raw);
  } catch {
    return failure(400, "", "invalid_json", "O corpo não é JSON válido.");
  }

  const tool = typeof body?.tool === "string" ? body.tool : "";
  if (!isFinanceToolId(tool)) {
    return failure(400, tool.slice(0, 80), "unknown_tool", `Ferramenta desconhecida: ${tool.slice(0, 80)}`);
  }

  const definition = FINANCE_TOOLS[tool];
  const parsed = definition.schema.safeParse(body?.args ?? {});
  if (!parsed.success) {
    const detail = parsed.error.issues
      .slice(0, 5)
      .map((issue) => `${issue.path.join(".") || "args"}: ${issue.message}`)
      .join("; ");
    return failure(400, tool, "invalid_args", `Argumentos inválidos — ${detail}`);
  }

  const owner = await getWhatsappOwnerUser();
  if (!owner) return failure(500, tool, "owner_not_configured", "Usuário autorizado não configurado.");

  let snapshot: Promise<FinanceSnapshot> | null = null;
  const context: ToolContext = {
    snapshot: () => (snapshot ??= loadFinanceSnapshot(owner.id)),
    liveInsights: async () => liveInsightsFor(await context.snapshot(), await getPreviousSafeToSpend(owner.id)),
    storedInsights: (limit) =>
      getStoredInsights(owner.id, limit).then((rows) =>
        rows.map((row) => ({
          id: row.id,
          type: row.type,
          severity: row.severity,
          status: row.status,
          title: row.title,
          summary: row.narrative ?? row.summary,
          firstSeenAt: row.firstSeenAt,
          lastSeenAt: row.lastSeenAt,
        })),
      ),
  };

  try {
    // O `run` é tipado pela ferramenta; aqui o args já passou pelo schema dela.
    const data = await (definition.run as (args: unknown, context: ToolContext) => Promise<unknown>)(
      parsed.data,
      context,
    );
    const payload = { ok: true, tool, asOf: new Date().toISOString(), data };
    const serialized = JSON.stringify(payload);
    if (Buffer.byteLength(serialized) > MAX_RESPONSE_BYTES) {
      return failure(413, tool, "response_too_large", "A resposta passou do limite; peça um recorte menor.");
    }
    return new Response(serialized, { status: 200, headers: JSON_UTF8 });
  } catch (error) {
    if (error instanceof FinanceDataUnavailableError) {
      return failure(503, tool, "db_unavailable", "Banco de dados indisponível.");
    }
    if (error instanceof FinanceAiNotConfiguredError) {
      return failure(422, tool, "ai_not_configured", "IA financeira não configurada no M-Finance.");
    }
    // Só o nome da ferramenta e o tipo do erro: payload financeiro não vai para log.
    console.error(`[mos/finance/query] ${tool} falhou: ${error instanceof Error ? error.name : "erro"}`);
    return failure(500, tool, "internal", "Falha ao calcular. Tente de novo.");
  }
}
