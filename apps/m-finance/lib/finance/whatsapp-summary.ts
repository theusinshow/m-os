import { and, eq, sql } from "drizzle-orm";
import { db } from "@/db/client";
import { creditCardExpenses } from "@/db/schema";
import { getBillsByMonth } from "@/lib/bills";
import { getCreditCards, getInvoicesByMonth } from "@/lib/cards";
import { getDashboardSummary } from "@/lib/calculations/dashboard";
import { formatCurrency } from "@/lib/formatters/currency";
import { getIncomesByMonth } from "@/lib/incomes";
import {
  getCurrentMonthForUser,
  getMonthByParts,
  getCurrentMonthParts,
  getMonthPartsAtOffset,
} from "@/lib/months";

function formatDate(value: string) {
  const [year, month, day] = value.split("-");

  if (!year || !month || !day) {
    return value;
  }

  return `${day}/${month}/${year}`;
}

function formatMonthName(month: number, year: number) {
  return new Intl.DateTimeFormat("pt-BR", {
    month: "long",
    year: "numeric",
    timeZone: "America/Sao_Paulo",
  }).format(new Date(year, month - 1, 1));
}

export async function getWhatsappMonthlySummary(userId: string) {
  const month = await getCurrentMonthForUser(userId);

  if (!month) {
    return "Ainda não existe mês atual criado no M Finance. Crie o mês atual pelo app antes de consultar pelo WhatsApp.";
  }

  const [incomes, bills, invoices] = await Promise.all([
    getIncomesByMonth(month.id),
    getBillsByMonth(month.id),
    getInvoicesByMonth(month.id),
  ]);

  const summary = getDashboardSummary({ incomes, bills, invoices });
  const label = formatMonthName(month.month, month.year);

  return [
    `Resumo de ${label}`,
    "",
    `Receitas: ${formatCurrency(summary.totalIncomeCents)}`,
    `Contas: ${formatCurrency(summary.totalBillsCents)}`,
    `Faturas: ${formatCurrency(summary.totalInvoicesCents)}`,
    `Pago: ${formatCurrency(summary.totalPaidCents)}`,
    `Pendente: ${formatCurrency(summary.totalPendingCents)}`,
    `Vencido: ${formatCurrency(summary.totalOverdueCents)}`,
    `Saldo estimado: ${formatCurrency(summary.estimatedRemainingCents)}`,
  ].join("\n");
}

export async function getWhatsappDueItems(userId: string) {
  const month = await getCurrentMonthForUser(userId);

  if (!month) {
    return "Ainda não existe mês atual criado no M Finance.";
  }

  const [bills, invoices] = await Promise.all([
    getBillsByMonth(month.id),
    getInvoicesByMonth(month.id),
  ]);

  const items = [
    ...bills.map((bill) => ({
      type: "Conta",
      name: bill.name,
      amountCents: bill.amountCents,
      dueDate: bill.dueDate,
      status: bill.status,
    })),
    ...invoices.map((invoice) => ({
      type: "Fatura",
      name: invoice.name,
      amountCents: invoice.amountCents,
      dueDate: invoice.dueDate,
      status: invoice.status,
    })),
  ]
    .filter((item) => item.status !== "paid")
    .sort((a, b) => a.dueDate.localeCompare(b.dueDate))
    .slice(0, 8);

  if (items.length === 0) {
    return "Nenhum vencimento pendente no mês atual.";
  }

  return [
    "Próximos vencimentos:",
    "",
    ...items.map(
      (item) =>
        `• ${item.type}: ${item.name} — ${formatCurrency(item.amountCents)} — ${formatDate(item.dueDate)} (${item.status === "overdue" ? "vencido" : "pendente"})`,
    ),
  ].join("\n");
}

function normalizeForLookup(value: string) {
  return value
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

const MONTH_NAMES = [
  "janeiro",
  "fevereiro",
  "marco",
  "abril",
  "maio",
  "junho",
  "julho",
  "agosto",
  "setembro",
  "outubro",
  "novembro",
  "dezembro",
];

/**
 * Tenta extrair um mês/ano da mensagem. Retorna o offset relativo ao mês
 * atual (0 = atual, 1 = próximo, -1 = anterior) ou null se não houver menção.
 * Aceita "agosto", "de agosto", "agosto de 2026", "mes que vem", "mes passado".
 */
export function parseMonthOffset(normalized: string): number | null {
  if (/\bmes\s+que\s+vem\b|\bproximo\s+mes\b/.test(normalized)) return 1;
  if (/\bmes\s+passado\b|\banterior\b/.test(normalized)) return -1;

  const match = /\b(?:de\s+)?(janeiro|fevereiro|marco|abril|maio|junho|julho|agosto|setembro|outubro|novembro|dezembro)(?:\s+de\s+(\d{4}))?\b/.exec(
    normalized,
  );
  if (match) {
    const monthIdx = MONTH_NAMES.indexOf(match[1]);
    if (monthIdx === -1) return null;
    const current = getCurrentMonthParts();
    const targetYear = match[2] ? Number(match[2]) : current.year;
    let offset = (targetYear - current.year) * 12 + (monthIdx + 1 - current.month);
    // "agosto" sem ano, se já passou, assume do ano que vem.
    if (!match[2] && offset < 0) offset += 12;
    return offset;
  }
  return null;
}

async function resolveCardByName(userId: string, hint: string) {
  const cards = await getCreditCards(userId);
  if (cards.length === 0) return null;
  const lookup = normalizeForLookup(hint);
  for (const card of cards) {
    if (normalizeForLookup(card.name) === lookup) return card;
  }
  for (const card of cards) {
    if (normalizeForLookup(card.name).includes(lookup) || lookup.includes(normalizeForLookup(card.name))) {
      return card;
    }
  }
  // Shorthand pj/pessoal
  if (/\bpj\b/.test(lookup)) {
    const business = cards.find((c) => c.cardType === "business");
    if (business) return business;
  }
  if (/\bpessoal\b/.test(lookup)) {
    const personal = cards.find((c) => c.cardType === "personal");
    if (personal) return personal;
  }
  return null;
}

/**
 * Resumo de um cartão específico: total gasto no mês e valor da fatura.
 * Responde a "quanto gastei no nubank", "fatura do itaú", "fatura do nubank de
 * agosto". Quando monthOffset é informado, consulta o mês relativo ao atual.
 */
export async function getWhatsappCardSummary(
  userId: string,
  cardHint: string,
  monthOffset = 0,
) {
  if (!db) return "Banco indisponível no momento.";

  let month = await getCurrentMonthForUser(userId);
  if (!month) return "Ainda não existe mês atual criado no M Finance.";

  if (monthOffset !== 0) {
    const parts = getMonthPartsAtOffset(month.month, month.year, monthOffset);
    const targetMonth = await getMonthByParts(userId, parts.month, parts.year);
    if (!targetMonth) {
      return `Ainda não existe mês de ${formatMonthName(parts.month, parts.year)} criado no app.`;
    }
    month = targetMonth;
  }

  const card = await resolveCardByName(userId, cardHint);
  if (!card) {
    const cards = await getCreditCards(userId);
    const list = cards.map((c) => `${c.name} (${c.cardType === "business" ? "PJ" : "pessoal"})`).join(", ");
    return `Não encontrei um cartão chamado "${cardHint}". Ativos: ${list}.`;
  }

  const invoice = (await getInvoicesByMonth(month.id)).find((r) => r.name === card.name);

  const [expenseRow] = await db
    .select({ total: sql<number>`coalesce(sum(${creditCardExpenses.amountCents}), 0)::int` })
    .from(creditCardExpenses)
    .where(
      and(
        eq(creditCardExpenses.userId, userId),
        eq(creditCardExpenses.cardId, card.id),
        eq(creditCardExpenses.monthId, month.id),
      ),
    );

  const total = Number(expenseRow?.total ?? 0);
  const label = formatMonthName(month.month, month.year);

  return [
    `${card.name} (${card.cardType === "business" ? "PJ" : "pessoal"}) — ${label}`,
    "",
    `Gastos no mês: ${formatCurrency(total)}`,
    invoice
      ? `Fatura: ${formatCurrency(invoice.amountCents)} — vence ${formatDate(invoice.dueDate)} (${invoice.status === "paid" ? "paga" : invoice.status === "overdue" ? "vencida" : "pendente"})`
      : "Fatura: ainda não gerada",
  ].join("\n");
}

/**
 * Lista apenas contas e faturas vencidas (status overdue) do mês atual.
 */
export async function getWhatsappOverdueItems(userId: string) {
  const month = await getCurrentMonthForUser(userId);
  if (!month) return "Ainda não existe mês atual criado no M Finance.";

  const [bills, invoices] = await Promise.all([
    getBillsByMonth(month.id),
    getInvoicesByMonth(month.id),
  ]);

  const items = [
    ...bills
      .filter((b) => b.status === "overdue")
      .map((b) => ({ type: "Conta", name: b.name, amountCents: b.amountCents, dueDate: b.dueDate })),
    ...invoices
      .filter((i) => i.status === "overdue")
      .map((i) => ({ type: "Fatura", name: i.name, amountCents: i.amountCents, dueDate: i.dueDate })),
  ].sort((a, b) => a.dueDate.localeCompare(b.dueDate));

  if (items.length === 0) return "Nenhuma conta ou fatura vencida no mês atual. 👍";

  return [
    "Vencidos:",
    "",
    ...items.map((i) => `• ${i.type}: ${i.name} — ${formatCurrency(i.amountCents)} — ${formatDate(i.dueDate)}`),
    "",
    `Total vencido: ${formatCurrency(items.reduce((acc, i) => acc + i.amountCents, 0))}`,
  ].join("\n");
}

/**
 * Compara o mês atual com o mês anterior: receitas, contas, faturas e saldo.
 * Destaca o que mudou de forma significativa.
 */
export async function getWhatsappMonthlyComparison(userId: string) {
  if (!db) return "Banco indisponível no momento.";

  const current = await getCurrentMonthForUser(userId);
  if (!current) return "Ainda não existe mês atual criado no M Finance.";

  const parts = getCurrentMonthParts();
  const prevDate = new Date(parts.year, parts.month - 2, 1);
  const prevMonth = await getMonthByParts(userId, prevDate.getMonth() + 1, prevDate.getFullYear());

  const [currentIncomes, currentBills, currentInvoices] = await Promise.all([
    getIncomesByMonth(current.id),
    getBillsByMonth(current.id),
    getInvoicesByMonth(current.id),
  ]);
  const currentSummary = getDashboardSummary({
    incomes: currentIncomes,
    bills: currentBills,
    invoices: currentInvoices,
  });

  if (!prevMonth) {
    const label = formatMonthName(current.month, current.year);
    return [
      `Comparação — ${label}`,
      "",
      "Sem mês anterior criado para comparar.",
      "",
      `Receitas: ${formatCurrency(currentSummary.totalIncomeCents)}`,
      `Contas: ${formatCurrency(currentSummary.totalBillsCents)}`,
      `Faturas: ${formatCurrency(currentSummary.totalInvoicesCents)}`,
      `Saldo estimado: ${formatCurrency(currentSummary.estimatedRemainingCents)}`,
    ].join("\n");
  }

  const [prevIncomes, prevBills, prevInvoices] = await Promise.all([
    getIncomesByMonth(prevMonth.id),
    getBillsByMonth(prevMonth.id),
    getInvoicesByMonth(prevMonth.id),
  ]);
  const prevSummary = getDashboardSummary({
    incomes: prevIncomes,
    bills: prevBills,
    invoices: prevInvoices,
  });

  const delta = (curr: number, prev: number) => curr - prev;
  const fmtDelta = (d: number) => (d > 0 ? `+${formatCurrency(d)}` : formatCurrency(d));

  const lines = [
    `Comparação — ${formatMonthName(current.month, current.year)} vs ${formatMonthName(prevMonth.month, prevMonth.year)}`,
    "",
    `Receitas: ${formatCurrency(currentSummary.totalIncomeCents)} (${fmtDelta(delta(currentSummary.totalIncomeCents, prevSummary.totalIncomeCents))})`,
    `Contas: ${formatCurrency(currentSummary.totalBillsCents)} (${fmtDelta(delta(currentSummary.totalBillsCents, prevSummary.totalBillsCents))})`,
    `Faturas: ${formatCurrency(currentSummary.totalInvoicesCents)} (${fmtDelta(delta(currentSummary.totalInvoicesCents, prevSummary.totalInvoicesCents))})`,
    `Pago: ${formatCurrency(currentSummary.totalPaidCents)} (${fmtDelta(delta(currentSummary.totalPaidCents, prevSummary.totalPaidCents))})`,
    `Saldo estimado: ${formatCurrency(currentSummary.estimatedRemainingCents)} (${fmtDelta(delta(currentSummary.estimatedRemainingCents, prevSummary.estimatedRemainingCents))})`,
  ];

  // Destaques: o que mudou mais de 20%.
  const highlights: string[] = [];
  const pct = (curr: number, prev: number) => (prev === 0 ? 0 : Math.round((curr / prev - 1) * 100));
  const billsPct = pct(currentSummary.totalBillsCents, prevSummary.totalBillsCents);
  const invoicesPct = pct(currentSummary.totalInvoicesCents, prevSummary.totalInvoicesCents);
  if (Math.abs(billsPct) >= 20) {
    highlights.push(`• Contas ${billsPct > 0 ? "subiram" : "caíram"} ${Math.abs(billsPct)}%`);
  }
  if (Math.abs(invoicesPct) >= 20) {
    highlights.push(`• Faturas ${invoicesPct > 0 ? "subiram" : "caíram"} ${Math.abs(invoicesPct)}%`);
  }
  if (highlights.length > 0) {
    lines.push("", "Destaques:", ...highlights);
  }

  return lines.join("\n");
}
