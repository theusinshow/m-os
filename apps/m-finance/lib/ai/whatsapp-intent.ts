import { z } from "zod";
import { env } from "@/lib/env";
import { getDeepSeekClient } from "@/lib/ai/deepseek";

const whatsappIntentSchema = z.discriminatedUnion("intent", [
  z.object({
    intent: z.literal("create_card_expense"),
    amountCents: z.number().int().positive(),
    description: z.string().trim().min(1),
    cardNameHint: z.string().trim().min(1).nullable(),
    purchaseDate: z
      .string()
      .regex(/^\d{4}-\d{2}-\d{2}$/)
      .nullable(),
    paymentType: z.enum(["cash", "installment"]),
    installments: z.number().int().min(2).max(60).nullable(),
    confidence: z.number().min(0).max(1),
  }),
  z.object({
    intent: z.literal("create_bill"),
    amountCents: z.number().int().positive(),
    description: z.string().trim().min(1),
    dueDay: z.number().int().min(1).max(31).nullable(),
    isRecurring: z.boolean(),
    confidence: z.number().min(0).max(1),
  }),
  z.object({
    intent: z.literal("mark_bill_paid"),
    description: z.string().trim().min(1),
    confidence: z.number().min(0).max(1),
  }),
  z.object({
    intent: z.literal("mark_invoice_paid"),
    cardNameHint: z.string().trim().min(1).nullable(),
    confidence: z.number().min(0).max(1),
  }),
  z.object({
    intent: z.literal("cancel_last_action"),
    confidence: z.number().min(0).max(1),
  }),
  z.object({
    intent: z.literal("edit_last_action"),
    newAmountCents: z.number().int().positive().nullable(),
    newDescription: z.string().trim().min(1).nullable(),
    confidence: z.number().min(0).max(1),
  }),
  z.object({
    intent: z.literal("unknown"),
    reason: z.string().trim().min(1),
    confidence: z.number().min(0).max(1),
  }),
]);

export type WhatsappIntent = z.infer<typeof whatsappIntentSchema>;

type IntentCardContext = {
  name: string;
  cardType: "personal" | "business";
};

function todayIso() {
  return new Intl.DateTimeFormat("en-CA", {
    timeZone: "America/Sao_Paulo",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(new Date());
}

function formatCardList(cards: IntentCardContext[]) {
  if (cards.length === 0) return "Nenhum cartão ativo cadastrado.";
  return cards
    .map((card) => `${card.name} (${card.cardType === "business" ? "PJ" : "pessoal"})`)
    .join(", ");
}

const cardExpenseExample = {
  intent: "create_card_expense",
  amountCents: 3290,
  description: "almoço",
  cardNameHint: "nubank pessoal",
  purchaseDate: "2026-07-03",
  paymentType: "cash",
  installments: null,
  confidence: 0.92,
};

const installmentExample = {
  intent: "create_card_expense",
  amountCents: 60000,
  description: "amazon",
  cardNameHint: "itaú",
  purchaseDate: "2026-07-03",
  paymentType: "installment",
  installments: 10,
  confidence: 0.9,
};

const billExample = {
  intent: "create_bill",
  amountCents: 12000,
  description: "luz",
  dueDay: null,
  isRecurring: false,
  confidence: 0.85,
};

export async function classifyWhatsappIntent(
  message: string,
  context?: { cards: IntentCardContext[] },
): Promise<WhatsappIntent> {
  const client = getDeepSeekClient();

  if (!client) {
    return {
      intent: "unknown",
      reason: "DeepSeek não configurado.",
      confidence: 0,
    };
  }

  const cards = context?.cards ?? [];

  const completion = await client.chat.completions.create({
    model: env.deepseekModel,
    messages: [
      {
        role: "system",
        content: [
          "Você é um extrator de intenção para um app financeiro pessoal chamado M Finance.",
          "Responda somente com JSON válido.",
          "Não execute ações. Apenas classifique a mensagem.",
          `A data de hoje no fuso America/Sao_Paulo é ${todayIso()}.`,
          `Cartões de crédito ativos do usuário: ${formatCardList(cards)}.`,
          "",
          "Regras de classificação:",
          "- Compra no cartão de crédito (menção a cartão, ou a 'no/na/pelo <cartão>' que casa com um cartão ativo) -> create_card_expense.",
          "- Despesa fora do cartão (em dinheiro, pix, débito, sem cartão, conta de luz/água/internet, boleto) -> create_bill.",
          "- Parcelamento ('em 6x', 'parcelado em 10 vezes') -> create_card_expense com paymentType installment e installments.",
          "- 'todo mês', 'assinatura', 'recorrente' -> create_bill com isRecurring true.",
          "- 'paguei a conta de X', 'marquei X como pago', 'já paguei X' (referindo-se a conta existente) -> mark_bill_paid.",
          "- 'paguei a fatura do X', 'marquei a fatura do itaú como paga' -> mark_invoice_paid.",
          "- 'cancela a última', 'desfaz o último lançamento', 'desfaz a última compra' -> cancel_last_action.",
          "- 'edita a última compra pra 50', 'muda a última para almoço' -> edit_last_action com newAmountCents e/ou newDescription.",
          "- Consulta, saudação, comando, ambígua ou sem gasto -> unknown.",
          "",
          "Conversões:",
          "- Valores em centavos BRL (32,90 -> 3290).",
          "- cardNameHint preserva qualificadores (pessoal, pj, business) e a marca; null se não mencionado.",
          "- \"ontem\" -> dia anterior à data de hoje; \"anteontem\" -> 2 dias antes; \"dia 15\" -> yyyy-mm-15.",
          "- Sem data explícita -> purchaseDate = hoje.",
          "- Sem dia de vencimento -> dueDay null.",
          "",
          "Exemplos de JSON:",
          JSON.stringify(cardExpenseExample),
          JSON.stringify(installmentExample),
          JSON.stringify(billExample),
        ].join("\n"),
      },
      {
        role: "user",
        content: `Classifique em json esta mensagem do WhatsApp: ${message}`,
      },
    ],
    response_format: { type: "json_object" },
    max_tokens: 500,
  });

  const content = completion.choices[0]?.message.content;

  if (!content) {
    return {
      intent: "unknown",
      reason: "DeepSeek retornou conteúdo vazio.",
      confidence: 0,
    };
  }

  try {
    return whatsappIntentSchema.parse(JSON.parse(content));
  } catch {
    return {
      intent: "unknown",
      reason: "DeepSeek retornou JSON fora do schema esperado.",
      confidence: 0,
    };
  }
}
