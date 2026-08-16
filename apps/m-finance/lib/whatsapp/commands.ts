import { classifyWhatsappIntent } from "@/lib/ai/whatsapp-intent";
import { getCreditCards } from "@/lib/cards";
import {
  getWhatsappCardSummary,
  getWhatsappDueItems,
  getWhatsappMonthlyComparison,
  getWhatsappMonthlySummary,
  getWhatsappOverdueItems,
  parseMonthOffset,
} from "@/lib/finance/whatsapp-summary";
import {
  getActiveWhatsappPendingAction,
  updateWhatsappPendingActionStatus,
} from "@/lib/whatsapp/audit";
import { executeWhatsappPendingAction } from "@/lib/whatsapp/action-executor";
import { tryHeuristicBill, tryHeuristicCancelLast, tryHeuristicCardExpense, tryHeuristicEditLast, tryHeuristicMarkInvoicePaid, tryHeuristicMarkPaid } from "@/lib/whatsapp/heuristics";
import {
  createPendingActionFromIntent,
  resolvePendingCardExpense,
} from "@/lib/whatsapp/pending-intents";
import { WHATSAPP_HELP_MESSAGE } from "@/lib/whatsapp/responses";

const CONFIRMABLE_ACTIONS = new Set([
  "create_card_expense",
  "create_bill",
  "mark_bill_paid",
  "mark_invoice_paid",
  "cancel_last_action",
  "edit_last_action",
]);

export async function handleWhatsappCommand({
  message,
  phone,
  userId,
}: {
  message: string;
  phone: string;
  userId: string;
}) {
  const normalized = message
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .trim()
    .toLowerCase();

  if (["sim", "s", "confirmar", "confirma"].includes(normalized)) {
    const pendingAction = await getActiveWhatsappPendingAction(userId, phone);

    if (!pendingAction) {
      return "Não encontrei nenhuma ação pendente para confirmar.";
    }

    if (pendingAction.actionType === "resolve_card_expense") {
      return "Ainda preciso que você responda com o cartão antes de confirmar.";
    }

    if (!CONFIRMABLE_ACTIONS.has(pendingAction.actionType)) {
      return "Essa ação ainda não pode ser confirmada por aqui.";
    }

    return executeWhatsappPendingAction(pendingAction);
  }

  if (["nao", "não", "n", "cancelar", "cancela"].includes(normalized)) {
    const pendingAction = await getActiveWhatsappPendingAction(userId, phone);

    if (!pendingAction) {
      return "Não encontrei nenhuma ação pendente para cancelar.";
    }

    await updateWhatsappPendingActionStatus(pendingAction.id, "cancelled");
    return "Ação pendente cancelada.";
  }

  const pendingAction = await getActiveWhatsappPendingAction(userId, phone);
  if (pendingAction?.actionType === "resolve_card_expense") {
    return resolvePendingCardExpense({ pendingAction, message });
  }

  if (!normalized || normalized === "ajuda" || normalized === "help") {
    return WHATSAPP_HELP_MESSAGE;
  }

  if (["resumo", "saldo", "gastos"].includes(normalized)) {
    return getWhatsappMonthlySummary(userId);
  }

  if (["vencimentos", "contas", "pendencias"].includes(normalized)) {
    return getWhatsappDueItems(userId);
  }

  if (["vencidas", "vencido", "atrasadas", "atrasado"].includes(normalized)) {
    return getWhatsappOverdueItems(userId);
  }

  if (["comparacao", "comparar", "mes passado", "mes anterior"].includes(normalized)) {
    return getWhatsappMonthlyComparison(userId);
  }

  // Consultas ricas por cartão: "quanto gastei no nubank", "fatura do itaú de
  // agosto", "saldo do nubank pj". Padrão determinístico — sem IA.
  const cardQueryMatch = normalized.match(
    /(?:quanto\s+(?:gastei|gasto)|gastos|fatura|saldo)\s+(?:no|na|do|da|de)\s+([a-z0-9]+(?:\s+[a-z0-9]+){0,2})/,
  );
  if (cardQueryMatch) {
    const monthOffset = parseMonthOffset(normalized) ?? 0;
    return getWhatsappCardSummary(userId, cardQueryMatch[1], monthOffset);
  }

  // Camada barata e determinística primeiro: resolve os padrões mais comuns de
  // despesa de cartão, despesa avulsa, marcação de pagamento e cancelamento,
  // sem gastar tokens da DeepSeek. Se nenhuma das duas tiver confiança, cai no
  // classificador de IA. Os cartões ativos são carregados uma única vez por
  // mensagem e reaproveitados por todos os passos.
  const cards = await getCreditCards(userId);
  const heuristicIntent =
    (await tryHeuristicCardExpense(message, cards)) ??
    (await tryHeuristicBill(message, cards)) ??
    (await tryHeuristicMarkPaid(message)) ??
    (await tryHeuristicMarkInvoicePaid(message)) ??
    (await tryHeuristicCancelLast(message)) ??
    (await tryHeuristicEditLast(message));
  const intent =
    heuristicIntent ?? (await classifyWhatsappIntent(message, { cards }));
  const pendingActionResult = await createPendingActionFromIntent({
    intent,
    message,
    phone,
    userId,
  });

  if (pendingActionResult) {
    return pendingActionResult.response;
  }

  return [
    "Ainda não entendi esse comando.",
    "",
    "Use `ajuda` para ver o que já está disponível.",
  ].join("\n");
}
