import { addMonths, effectiveStatus, monthKey } from "@/lib/finance-intelligence/dates";
import { normalizeForMatch, sanitizeLabel } from "@/lib/finance-intelligence/sanitize";
import type { FinanceSnapshot } from "@/lib/finance-intelligence/types";

export const ENTITY_KINDS = ["bill", "invoice", "card", "subscription", "goal", "income"] as const;
export type EntityKind = (typeof ENTITY_KINDS)[number];

export type EntityMatch = {
  kind: EntityKind;
  id: string;
  label: string;
  amountCents: number | null;
  date: string | null;
  status: string | null;
  month: string | null;
  /** Para ação sobre fatura, o M/OS precisa do cartão, não só da fatura. */
  cardId?: string;
};

export const MAX_ENTITY_MATCHES = 12;

/**
 * Acha entidades pelo nome, com id — o degrau que falta entre "paguei a
 * internet" e `m-finance.mark_bill_paid { billId }`.
 *
 * Contas e faturas procuradas numa janela curta (mês passado, atual e
 * próximo): "a internet" é a desta vez, não a de 2025. Todo termo da busca
 * precisa aparecer no nome — "conta luz" acha "Conta de luz", e não "Conta de
 * água" só por causa de "conta".
 */
export function findEntities(
  snapshot: FinanceSnapshot,
  query: string,
  kinds: readonly EntityKind[] = ENTITY_KINDS,
) {
  const terms = normalizeForMatch(query).split(" ").filter(Boolean);
  const matches = (name: string) => {
    const haystack = normalizeForMatch(name);
    return terms.length > 0 && terms.every((term) => haystack.includes(term));
  };
  const window = new Set(
    [-1, 0, 1].map((offset) => monthKey(addMonths(snapshot.current, offset))),
  );
  const found: EntityMatch[] = [];

  if (kinds.includes("bill")) {
    for (const bill of snapshot.bills) {
      if (!window.has(monthKey(bill)) || !matches(bill.name)) continue;
      found.push({
        kind: "bill",
        id: bill.id,
        label: sanitizeLabel(bill.name),
        amountCents: bill.amountCents,
        date: bill.dueDate,
        status: effectiveStatus(bill.status, bill.dueDate, snapshot.today),
        month: monthKey(bill),
      });
    }
  }

  const matchedCards = snapshot.cards.filter((card) => matches(card.name));
  if (kinds.includes("card")) {
    for (const card of matchedCards) {
      found.push({
        kind: "card",
        id: card.id,
        label: sanitizeLabel(card.name),
        amountCents: null,
        date: null,
        status: card.isActive ? "active" : "inactive",
        month: null,
        cardId: card.id,
      });
    }
  }
  if (kinds.includes("invoice")) {
    for (const invoice of snapshot.invoices) {
      const card = matchedCards.find((row) => row.id === invoice.cardId);
      if (!card || !window.has(monthKey(invoice))) continue;
      found.push({
        kind: "invoice",
        id: invoice.id,
        label: `Fatura ${sanitizeLabel(card.name)}`,
        amountCents: invoice.amountCents,
        date: invoice.dueDate,
        status: effectiveStatus(invoice.status, invoice.dueDate, snapshot.today),
        month: monthKey(invoice),
        cardId: card.id,
      });
    }
  }
  if (kinds.includes("subscription")) {
    for (const row of snapshot.subscriptions) {
      if (!matches(row.name)) continue;
      found.push({
        kind: "subscription",
        id: row.id,
        label: sanitizeLabel(row.name),
        amountCents: row.amountCents,
        date: row.nextChargeDate,
        status: row.status,
        month: null,
      });
    }
  }
  if (kinds.includes("goal")) {
    for (const row of snapshot.goals) {
      if (row.status === "archived" || !matches(row.name)) continue;
      found.push({
        kind: "goal",
        id: row.id,
        label: sanitizeLabel(row.name),
        amountCents: row.targetAmountCents,
        date: row.deadline,
        status: row.status,
        month: null,
      });
    }
  }
  if (kinds.includes("income")) {
    for (const row of snapshot.incomes) {
      if (!window.has(monthKey(row)) || !matches(row.name)) continue;
      found.push({
        kind: "income",
        id: row.id,
        label: sanitizeLabel(row.name),
        amountCents: row.amountCents,
        date: row.expectedDate,
        status: row.received ? "received" : "expected",
        month: monthKey(row),
      });
    }
  }

  // Não pago antes de pago, mês atual antes dos outros: o candidato mais
  // provável de "paguei a X" é a X pendente deste mês.
  const current = monthKey(snapshot.current);
  const rank = (match: EntityMatch) =>
    (match.status === "paid" ? 2 : 0) + (match.month && match.month !== current ? 1 : 0);
  found.sort((a, b) => rank(a) - rank(b));

  return {
    query: sanitizeLabel(query),
    count: found.length,
    matches: found.slice(0, MAX_ENTITY_MATCHES),
    truncated: found.length > MAX_ENTITY_MATCHES,
  };
}
