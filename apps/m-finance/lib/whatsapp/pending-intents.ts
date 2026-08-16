import { z } from "zod";
import { getCreditCards } from "@/lib/cards";
import { formatCurrency } from "@/lib/formatters/currency";
import type { WhatsappIntent } from "@/lib/ai/whatsapp-intent";
import { createWhatsappPendingAction, updateWhatsappPendingActionStatus } from "@/lib/whatsapp/audit";

function normalize(value: string) {
  return value
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

function tokenize(value: string) {
  return normalize(value).split(" ").filter(Boolean);
}

function formatDate(value: string | null) {
  if (!value) return "hoje";
  const [year, month, day] = value.split("-");
  if (!year || !month || !day) return value;
  return `${day}/${month}/${year}`;
}

async function resolveCard(userId: string, cardNameHint: string | null, originalMessage: string) {
  const cards = await getCreditCards(userId);

  if (cards.length === 0) {
    return { card: null, reason: "Nenhum cartão ativo encontrado no app." };
  }

  if (!cardNameHint) {
    if (cards.length === 1) {
      return { card: cards[0], reason: null };
    }

    return {
      card: null,
      reason: `Qual cartão devo usar? Ativos: ${cards.map((card) => card.name).join(", ")}.`,
    };
  }

  const lookupText = normalize([cardNameHint, originalMessage].filter(Boolean).join(" "));
  const lookupTokens = new Set(tokenize(lookupText));
  const hint = normalize(cardNameHint);

  const scoredCards = cards
    .map((card) => {
      const cardName = normalize(card.name);
      const cardTokens = tokenize(card.name);
      let score = 0;

      if (cardName === hint) score += 100;
      if (lookupText.includes(cardName)) score += 80;
      if (hint && cardName.includes(hint)) score += 40;
      if (hint && hint.includes(cardName)) score += 40;

      for (const token of cardTokens) {
        if (lookupTokens.has(token)) score += token.length > 2 ? 10 : 2;
      }

      // Common local shorthand: "PJ" maps to business cards and "pessoal" to
      // personal cards even when the AI only returns the base brand as hint.
      if (lookupTokens.has("pj") && card.cardType === "business") score += 30;
      if (lookupTokens.has("pessoal") && card.cardType === "personal") score += 30;
      if (lookupTokens.has("empresa") && card.cardType === "business") score += 20;
      if (lookupTokens.has("business") && card.cardType === "business") score += 20;

      return { card, score };
    })
    .filter((item) => item.score > 0)
    .sort((a, b) => b.score - a.score);

  const best = scoredCards[0];
  const second = scoredCards[1];

  if (best && (!second || best.score > second.score)) {
    return { card: best.card, reason: null };
  }

  return {
    card: null,
    reason:
      scoredCards.length > 1
        ? `Encontrei mais de um cartão para "${cardNameHint}". Seja mais específico.`
        : `Não encontrei cartão ativo parecido com "${cardNameHint}". Ativos: ${cards.map((card) => card.name).join(", ")}.`,
  };
}

export async function resolvePendingCardExpense({
  pendingAction,
  message,
}: {
  pendingAction: { id: string; userId: string; phone: string; payload: unknown };
  message: string;
}) {
  const basePayload = pendingCardPayloadSchema.safeParse(pendingAction.payload);
  if (!basePayload.success) {
    await updateWhatsappPendingActionStatus(pendingAction.id, "cancelled");
    return "A pendência estava inválida. Envie o lançamento novamente.";
  }

  const { card, reason } = await resolveCard(pendingAction.userId, message, message);
  if (!card) return reason ?? "Não consegui identificar o cartão.";

  await updateWhatsappPendingActionStatus(pendingAction.id, "cancelled");

  return createCardExpensePendingAction({
    userId: pendingAction.userId,
    phone: pendingAction.phone,
    payload: {
      ...basePayload.data,
      cardId: card.id,
      cardName: card.name,
    },
  });
}

const pendingCardPayloadSchema = z.object({
  amountCents: z.number().int().positive(),
  description: z.string().trim().min(1),
  purchaseDate: z.string().regex(/^\d{4}-\d{2}-\d{2}$/).nullable(),
  paymentType: z.enum(["cash", "installment"]),
  installments: z.number().int().min(2).max(60).nullable(),
  confidence: z.number().min(0).max(1),
});

const pendingBillPayloadSchema = z.object({
  amountCents: z.number().int().positive(),
  description: z.string().trim().min(1),
  dueDay: z.number().int().min(1).max(31).nullable(),
  isRecurring: z.boolean(),
  confidence: z.number().min(0).max(1),
});

const pendingMarkPaidPayloadSchema = z.object({
  description: z.string().trim().min(1),
  confidence: z.number().min(0).max(1),
});

const pendingMarkInvoicePaidPayloadSchema = z.object({
  cardId: z.string().uuid(),
  cardName: z.string().trim().min(1),
  confidence: z.number().min(0).max(1),
});

const pendingCancelLastPayloadSchema = z.object({
  confidence: z.number().min(0).max(1),
});

const pendingEditLastPayloadSchema = z.object({
  newAmountCents: z.number().int().positive().nullable(),
  newDescription: z.string().trim().min(1).nullable(),
  confidence: z.number().min(0).max(1),
});

async function createCardExpensePendingAction({
  userId,
  phone,
  payload,
}: {
  userId: string;
  phone: string;
  payload: z.infer<typeof pendingCardPayloadSchema> & { cardId: string; cardName: string };
}) {
  const summary = [
    "Confirmar lançamento?",
    "",
    `Compra no cartão: ${formatCurrency(payload.amountCents)}`,
    `Descrição: ${payload.description}`,
    `Cartão: ${payload.cardName}`,
    `Data: ${formatDate(payload.purchaseDate)}`,
    payload.paymentType === "installment" && payload.installments
      ? `Parcelamento: ${payload.installments}x`
      : null,
    "",
    "Responda sim ou não.",
  ]
    .filter(Boolean)
    .join("\n");

  const pendingAction = await createWhatsappPendingAction({
    userId,
    phone,
    actionType: "create_card_expense",
    summary,
    payload,
  });

  return pendingAction
    ? summary
    : "Não consegui criar a ação pendente agora.";
}

async function createMarkPaidPendingAction({
  userId,
  phone,
  payload,
}: {
  userId: string;
  phone: string;
  payload: z.infer<typeof pendingMarkPaidPayloadSchema>;
}) {
  const parsed = pendingMarkPaidPayloadSchema.safeParse(payload);
  if (!parsed.success) {
    return "Não consegui interpretar essa marcação de pagamento. Tente novamente.";
  }
  const bill = parsed.data;
  const summary = [
    "Confirmar marcação de pagamento?",
    "",
    `Conta: ${bill.description}`,
    "",
    "Responda sim ou não.",
  ].join("\n");

  const pendingAction = await createWhatsappPendingAction({
    userId,
    phone,
    actionType: "mark_bill_paid",
    summary,
    payload: bill,
  });

  return pendingAction
    ? summary
    : "Não consegui criar a ação pendente agora.";
}

async function createCancelLastPendingAction({
  userId,
  phone,
  payload,
}: {
  userId: string;
  phone: string;
  payload: z.infer<typeof pendingCancelLastPayloadSchema>;
}) {
  const parsed = pendingCancelLastPayloadSchema.safeParse(payload);
  if (!parsed.success) {
    return "Não consegui interpretar esse cancelamento. Tente novamente.";
  }
  const summary = [
    "Confirmar cancelamento?",
    "",
    "Vou desfazer o último lançamento confirmado por aqui.",
    "",
    "Responda sim ou não.",
  ].join("\n");

  const pendingAction = await createWhatsappPendingAction({
    userId,
    phone,
    actionType: "cancel_last_action",
    summary,
    payload: parsed.data,
  });

  return pendingAction
    ? summary
    : "Não consegui criar a ação pendente agora.";
}

async function createMarkInvoicePaidPendingAction({
  userId,
  phone,
  payload,
}: {
  userId: string;
  phone: string;
  payload: z.infer<typeof pendingMarkInvoicePaidPayloadSchema>;
}) {
  const parsed = pendingMarkInvoicePaidPayloadSchema.safeParse(payload);
  if (!parsed.success) {
    return "Não consegui interpretar essa marcação de fatura. Tente novamente.";
  }
  const invoice = parsed.data;
  const summary = [
    "Confirmar marcação de pagamento?",
    "",
    `Fatura: ${invoice.cardName}`,
    "",
    "Responda sim ou não.",
  ].join("\n");

  const pendingAction = await createWhatsappPendingAction({
    userId,
    phone,
    actionType: "mark_invoice_paid",
    summary,
    payload: invoice,
  });

  return pendingAction
    ? summary
    : "Não consegui criar a ação pendente agora.";
}

async function createEditLastPendingAction({
  userId,
  phone,
  payload,
}: {
  userId: string;
  phone: string;
  payload: z.infer<typeof pendingEditLastPayloadSchema>;
}) {
  const parsed = pendingEditLastPayloadSchema.safeParse(payload);
  if (!parsed.success) {
    return "Não consegui interpretar essa edição. Tente novamente.";
  }
  const edit = parsed.data;
  const changes: string[] = [];
  if (edit.newAmountCents) changes.push(`Novo valor: ${formatCurrency(edit.newAmountCents)}`);
  if (edit.newDescription) changes.push(`Nova descrição: ${edit.newDescription}`);

  if (changes.length === 0) {
    return "Não entendi o que quer editar. Diga o novo valor ou descrição.";
  }

  const summary = [
    "Confirmar edição do último lançamento?",
    "",
    ...changes,
    "",
    "Responda sim ou não.",
  ].join("\n");

  const pendingAction = await createWhatsappPendingAction({
    userId,
    phone,
    actionType: "edit_last_action",
    summary,
    payload: edit,
  });

  return pendingAction
    ? summary
    : "Não consegui criar a ação pendente agora.";
}

async function createBillPendingAction({
  userId,
  phone,
  payload,
}: {
  userId: string;
  phone: string;
  payload: z.infer<typeof pendingBillPayloadSchema>;
}) {
  const parsed = pendingBillPayloadSchema.safeParse(payload);
  if (!parsed.success) {
    return "Não consegui interpretar essa despesa avulsa. Tente enviar novamente.";
  }

  const bill = parsed.data;
  const dueLabel = bill.dueDay ? `dia ${bill.dueDay}` : "fim do mês";
  const summary = [
    "Confirmar lançamento?",
    "",
    `Despesa avulsa: ${formatCurrency(bill.amountCents)}`,
    `Descrição: ${bill.description}`,
    `Vencimento: ${dueLabel}`,
    bill.isRecurring && bill.dueDay ? `Recorrência: sim (próximos 12 meses)` : null,
    bill.isRecurring && !bill.dueDay ? "Recorrente: sim (apenas este mês)" : null,
    "",
    "Responda sim ou não.",
  ]
    .filter(Boolean)
    .join("\n");

  const pendingAction = await createWhatsappPendingAction({
    userId,
    phone,
    actionType: "create_bill",
    summary,
    payload: bill,
  });

  return pendingAction
    ? summary
    : "Não consegui criar a ação pendente agora.";
}

export async function createPendingActionFromIntent({
  intent,
  message,
  phone,
  userId,
}: {
  intent: WhatsappIntent;
  message: string;
  phone: string;
  userId: string;
}) {
  if (intent.intent === "create_bill") {
    const response = await createBillPendingAction({
      userId,
      phone,
      payload: {
        amountCents: intent.amountCents,
        description: intent.description,
        dueDay: intent.dueDay,
        isRecurring: intent.isRecurring,
        confidence: intent.confidence,
      },
    });

    return {
      created: true as const,
      response,
    };
  }

  if (intent.intent === "mark_bill_paid") {
    const response = await createMarkPaidPendingAction({
      userId,
      phone,
      payload: {
        description: intent.description,
        confidence: intent.confidence,
      },
    });

    return {
      created: true as const,
      response,
    };
  }

  if (intent.intent === "mark_invoice_paid") {
    // Resolve o cartão a partir do cardNameHint antes de criar a pendência.
    const { card, reason } = await resolveCard(userId, intent.cardNameHint, message);
    if (!card) {
      return {
        created: true as const,
        response: `${reason ?? "Não encontrei a fatura."}\n\nResponda com o nome do cartão para continuar.`,
      };
    }

    const response = await createMarkInvoicePaidPendingAction({
      userId,
      phone,
      payload: {
        cardId: card.id,
        cardName: card.name,
        confidence: intent.confidence,
      },
    });

    return {
      created: true as const,
      response,
    };
  }

  if (intent.intent === "cancel_last_action") {
    const response = await createCancelLastPendingAction({
      userId,
      phone,
      payload: { confidence: intent.confidence },
    });

    return {
      created: true as const,
      response,
    };
  }

  if (intent.intent === "edit_last_action") {
    const response = await createEditLastPendingAction({
      userId,
      phone,
      payload: {
        newAmountCents: intent.newAmountCents,
        newDescription: intent.newDescription,
        confidence: intent.confidence,
      },
    });

    return {
      created: true as const,
      response,
    };
  }

  if (intent.intent !== "create_card_expense") {
    return null;
  }

  const { card, reason } = await resolveCard(userId, intent.cardNameHint, message);

  if (!card) {
    await createWhatsappPendingAction({
      userId,
      phone,
      actionType: "resolve_card_expense",
      summary: "Aguardando seleção de cartão para lançamento.",
      payload: {
        amountCents: intent.amountCents,
        description: intent.description,
        purchaseDate: intent.purchaseDate,
        paymentType: intent.paymentType,
        installments: intent.installments,
        confidence: intent.confidence,
      },
    });

    return {
      created: true as const,
      response: `${reason ?? "Não consegui identificar o cartão."}\n\nResponda com o nome do cartão para continuar.`,
    };
  }

  const response = await createCardExpensePendingAction({
    userId,
    phone,
    payload: {
      amountCents: intent.amountCents,
      description: intent.description,
      cardId: card.id,
      cardName: card.name,
      purchaseDate: intent.purchaseDate,
      paymentType: intent.paymentType,
      installments: intent.installments,
      confidence: intent.confidence,
    },
  });

  return {
    created: true as const,
    response,
  };
}
