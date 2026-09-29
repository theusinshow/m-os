import { z } from "zod";
import { createCardExpense } from "@/lib/domain/finance-actions/create-card-expense";
import {
  createGoalEntry,
  createIncomeEntry,
  createSubscriptionEntry,
  setFinancialPolicy,
  updateGoalEntry,
} from "@/lib/domain/finance-actions/entries";
import { markBillPaid, markInvoicePaid } from "@/lib/domain/finance-actions/mark-paid";
import type { DomainResult } from "@/lib/domain/finance-actions/result";
import { getCardById } from "@/lib/card-expenses";
import { parseMonthKey } from "@/lib/finance-intelligence/dates";
import { POLICY_KEYS, POLICY_LABELS } from "@/lib/finance-intelligence/policies";
import { normalizeForMatch } from "@/lib/finance-intelligence/sanitize";
import { formatCurrency } from "@/lib/formatters/currency";
import { createBillFromMosAction } from "@/lib/mos/action-bridge";
import { ensureMonthForUser, getCurrentMonthParts } from "@/lib/months";

/**
 * O catálogo de ESCRITA que o M/OS pode executar, depois do preview e da
 * confirmação do dono. Uma entrada por ação, com schema próprio; nenhuma ação
 * genérica. Toda ação mexe em dinheiro, então toda ação é risco alto — quem
 * decide isso no M/OS é `functions.rs`, e este lado só executa o que chegou.
 *
 * Cada executor chama o MESMO serviço de domínio que a web e o WhatsApp usam.
 */

export type ReceiptEntity = { type: string; id: string; label: string };

export type MosActionReceipt = {
  actionId: string;
  message: string;
  entities: ReceiptEntity[];
  executedAt: string;
};

export type MosActionOutcome =
  | { ok: true; receipt: MosActionReceipt; billId?: string }
  | { ok: false; error: string; code?: string };

const date = z.string().regex(/^\d{4}-\d{2}-\d{2}$/, "Use AAAA-MM-DD.");
const month = z.string().regex(/^\d{4}-(0[1-9]|1[0-2])$/, "Use AAAA-MM.");
const cents = z.number().int().positive().max(100_000_000);
const text = z.string().trim().min(1).max(120);

export const actionSchemas = {
  "m-finance.create_card_expense": z
    .object({
      cardId: z.string().uuid(),
      cardName: text,
      amountCents: cents,
      description: text,
      installments: z.number().int().min(1).max(60).default(1),
      purchaseDate: date.nullable().optional(),
    })
    .strict()
    .refine((value) => value.installments === 1 || value.amountCents >= value.installments, {
      message: "O valor total é muito baixo para essa quantidade de parcelas.",
    }),
  "m-finance.create_income": z
    .object({
      name: text,
      amountCents: cents,
      incomeType: z.enum(["main", "extra", "freelance"]),
      month: month.optional(),
      expectedDate: date.nullable().optional(),
      received: z.boolean().default(false),
    })
    .strict(),
  "m-finance.mark_bill_paid": z
    .object({ billId: z.string().uuid(), billName: text, amountCents: cents })
    .strict(),
  "m-finance.mark_invoice_paid": z
    .object({
      cardId: z.string().uuid(),
      cardName: text,
      month: month.optional(),
      amountCents: cents.optional(),
    })
    .strict(),
  "m-finance.create_subscription": z
    .object({
      name: text,
      amountCents: cents,
      nextChargeDate: date,
      cycle: z.enum(["once", "monthly", "yearly"]).default("monthly"),
      isTrial: z.boolean().default(false),
      reminderDaysBefore: z.number().int().min(0).max(30).default(1),
    })
    .strict(),
  "m-finance.create_goal": z
    .object({
      name: text,
      targetAmountCents: cents,
      currentAmountCents: z.number().int().min(0).max(100_000_000).default(0),
      deadline: date.nullable().optional(),
      priority: z.enum(["low", "medium", "high"]).default("medium"),
    })
    .strict(),
  "m-finance.update_goal": z
    .object({
      goalId: z.string().uuid(),
      goalName: text,
      name: text.optional(),
      targetAmountCents: cents.optional(),
      currentAmountCents: z.number().int().min(0).max(100_000_000).optional(),
      deadline: date.nullable().optional(),
      priority: z.enum(["low", "medium", "high"]).optional(),
    })
    .strict()
    .refine(
      (value) =>
        value.name !== undefined ||
        value.targetAmountCents !== undefined ||
        value.currentAmountCents !== undefined ||
        value.deadline !== undefined ||
        value.priority !== undefined,
      { message: "A edição não muda nada." },
    ),
  "m-finance.set_policy": z
    .object({ key: z.enum(POLICY_KEYS as [string, ...string[]]), value: z.unknown() })
    .strict(),
} as const;

type Args<K extends keyof typeof actionSchemas> = z.infer<(typeof actionSchemas)[K]>;

type SchemaActionId = keyof typeof actionSchemas;
export type MosActionId = "m-finance.create_bill" | SchemaActionId;
export const MOS_ACTION_IDS: readonly MosActionId[] = [
  "m-finance.create_bill",
  ...(Object.keys(actionSchemas) as SchemaActionId[]),
];

export function isMosActionId(value: unknown): value is MosActionId {
  return typeof value === "string" && (MOS_ACTION_IDS as readonly string[]).includes(value);
}

function receipt(
  actionId: string,
  message: string,
  entities: ReceiptEntity[],
): { ok: true; receipt: MosActionReceipt } {
  return { ok: true, receipt: { actionId, message, entities, executedAt: new Date().toISOString() } };
}

function refused<T>(result: Extract<DomainResult<T>, { ok: false }>): MosActionOutcome {
  return { ok: false, error: result.message, code: result.code };
}

async function currentMonthRecord(userId: string, key?: string) {
  const parts = (key && parseMonthKey(key)) || getCurrentMonthParts();
  return ensureMonthForUser(userId, parts.month, parts.year);
}

/**
 * Executa uma ação já confirmada. Argumento fora do schema é recusado, nunca
 * corrigido — corrigir seria o M-Finance adivinhando o que o preview mostrou.
 */
export async function executeMosAction(actionId: MosActionId, userId: string, rawArgs: unknown): Promise<MosActionOutcome> {
  if (actionId === "m-finance.create_bill") {
    const result = await createBillFromMosAction(userId, rawArgs);
    if (!result.ok) return { ok: false, error: result.error };
    const args = rawArgs as { description: string; amountCents: number };
    return {
      ...receipt(actionId, `Conta "${args.description}" de ${formatCurrency(args.amountCents)} criada no M-Finance.`, [
        { type: "bill", id: result.billId, label: args.description },
      ]),
      billId: result.billId,
    };
  }

  const parsed = actionSchemas[actionId].safeParse(rawArgs);
  if (!parsed.success) {
    return { ok: false, error: "Os argumentos da ação não batem com o esperado.", code: "invalid" };
  }

  switch (actionId) {
    case "m-finance.create_card_expense": {
      const args = parsed.data as Args<typeof actionId>;
      const card = await getCardById(userId, args.cardId);
      if (!card) return { ok: false, error: "Cartão não encontrado no M-Finance.", code: "not_found" };
      if (normalizeForMatch(card.name) !== normalizeForMatch(args.cardName)) {
        return { ok: false, error: `O cartão mudou desde o preview ("${card.name}"). Peça de novo.`, code: "stale_preview" };
      }
      const result = await createCardExpense({
        userId,
        card: { id: card.id, name: card.name, dueDay: card.dueDay },
        month: await currentMonthRecord(userId),
        description: args.description,
        amountCents: args.amountCents,
        purchaseDate: args.purchaseDate ?? null,
        installments: args.installments,
      });
      if (!result.ok) return refused(result);
      const split = args.installments > 1 ? ` em ${args.installments}x` : "";
      return receipt(actionId, `Compra "${args.description}" de ${formatCurrency(args.amountCents)}${split} lançada no ${card.name}.`, [
        { type: "card_expense", id: result.value.expenseIds[0] ?? "", label: args.description },
        { type: "card", id: card.id, label: card.name },
      ]);
    }
    case "m-finance.create_income": {
      const args = parsed.data as Args<typeof actionId>;
      const monthRecord = await currentMonthRecord(userId, args.month);
      const result = await createIncomeEntry({
        userId,
        month: monthRecord,
        name: args.name,
        amountCents: args.amountCents,
        incomeType: args.incomeType,
        expectedDate: args.expectedDate ?? null,
        received: args.received,
      });
      if (!result.ok) return refused(result);
      return receipt(
        actionId,
        `Receita "${args.name}" de ${formatCurrency(args.amountCents)} lançada em ${String(monthRecord.month).padStart(2, "0")}/${monthRecord.year}.`,
        [{ type: "income", id: result.value.id, label: args.name }],
      );
    }
    case "m-finance.mark_bill_paid": {
      const args = parsed.data as Args<typeof actionId>;
      const result = await markBillPaid(userId, args.billId, { amountCents: args.amountCents });
      if (!result.ok) return refused(result);
      return receipt(actionId, `Conta "${result.value.name}" (${formatCurrency(result.value.amountCents)}) marcada como paga.`, [
        { type: "bill", id: result.value.id, label: result.value.name },
      ]);
    }
    case "m-finance.mark_invoice_paid": {
      const args = parsed.data as Args<typeof actionId>;
      const monthRecord = await currentMonthRecord(userId, args.month);
      const result = await markInvoicePaid(
        userId,
        { cardId: args.cardId, monthId: monthRecord.id },
        { amountCents: args.amountCents ?? null },
      );
      if (!result.ok) return refused(result);
      return receipt(actionId, `Fatura ${args.cardName} (${formatCurrency(result.value.amountCents)}) marcada como paga.`, [
        { type: "invoice", id: result.value.id, label: `Fatura ${args.cardName}` },
      ]);
    }
    case "m-finance.create_subscription": {
      const args = parsed.data as Args<typeof actionId>;
      const result = await createSubscriptionEntry({ userId, ...args });
      if (!result.ok) return refused(result);
      return receipt(
        actionId,
        `${args.isTrial ? "Teste grátis" : "Assinatura"} "${args.name}" salva; próxima cobrança em ${args.nextChargeDate}.`,
        [{ type: "subscription", id: result.value.id, label: args.name }],
      );
    }
    case "m-finance.create_goal": {
      const args = parsed.data as Args<typeof actionId>;
      const result = await createGoalEntry(userId, {
        name: args.name,
        targetAmountCents: args.targetAmountCents,
        currentAmountCents: args.currentAmountCents,
        deadline: args.deadline ?? null,
        priority: args.priority,
      });
      if (!result.ok) return refused(result);
      return receipt(actionId, `Meta "${args.name}" de ${formatCurrency(args.targetAmountCents)} criada.`, [
        { type: "goal", id: result.value.id, label: args.name },
      ]);
    }
    case "m-finance.update_goal": {
      const args = parsed.data as Args<typeof actionId>;
      const result = await updateGoalEntry(
        userId,
        args.goalId,
        {
          name: args.name,
          targetAmountCents: args.targetAmountCents,
          currentAmountCents: args.currentAmountCents,
          deadline: args.deadline,
          priority: args.priority,
        },
        { name: args.goalName },
      );
      if (!result.ok) return refused(result);
      return receipt(actionId, `Meta "${result.value.name}" atualizada.`, [
        { type: "goal", id: result.value.id, label: result.value.name },
      ]);
    }
    case "m-finance.set_policy": {
      const args = parsed.data as Args<typeof actionId>;
      const key = args.key as (typeof POLICY_KEYS)[number];
      const result = await setFinancialPolicy(userId, key, args.value, "hermes");
      if (!result.ok) return refused(result);
      return receipt(actionId, `Política "${POLICY_LABELS[key]}" atualizada.`, [
        { type: "policy", id: key, label: POLICY_LABELS[key] },
      ]);
    }
  }

  return { ok: false, error: "Ação sem execução implementada.", code: "invalid" };
}
