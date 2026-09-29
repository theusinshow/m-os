import type { MonthParts, PayableStatus } from "@/lib/finance-intelligence/types";

/**
 * Aritmética de mês e dia sem `Date` local.
 *
 * O kernel roda no servidor da Vercel, em UTC, e fala de datas civis do dono
 * (`yyyy-mm-dd`). Comparar string ISO é exato; montar `new Date()` no fuso do
 * servidor é o jeito clássico de um vencimento "hoje" virar "ontem" às 21h.
 */

export function monthIndex({ month, year }: MonthParts) {
  return year * 12 + (month - 1);
}

export function fromMonthIndex(index: number): MonthParts {
  return { month: (((index % 12) + 12) % 12) + 1, year: Math.floor(index / 12) };
}

export function addMonths(parts: MonthParts, offset: number): MonthParts {
  return fromMonthIndex(monthIndex(parts) + offset);
}

export function sameMonth(a: MonthParts, b: MonthParts) {
  return a.month === b.month && a.year === b.year;
}

export function monthKey({ month, year }: MonthParts) {
  return `${year}-${String(month).padStart(2, "0")}`;
}

export function parseMonthKey(value: string): MonthParts | null {
  const match = /^(\d{4})-(\d{2})$/.exec(value.trim());
  if (!match) return null;
  const month = Number(match[2]);
  const year = Number(match[1]);
  if (month < 1 || month > 12 || year < 2020 || year > 2100) return null;
  return { month, year };
}

export function monthOfDate(isoDate: string): MonthParts {
  const [year, month] = isoDate.split("-").map(Number);
  return { month, year };
}

/** Dias civis entre duas datas `yyyy-mm-dd` (b − a). */
export function daysBetween(a: string, b: string) {
  const toUtc = (iso: string) => {
    const [year, month, day] = iso.split("-").map(Number);
    return Date.UTC(year, month - 1, day);
  };
  return Math.round((toUtc(b) - toUtc(a)) / 86_400_000);
}

export function addDays(isoDate: string, days: number) {
  const [year, month, day] = isoDate.split("-").map(Number);
  const date = new Date(Date.UTC(year, month - 1, day + days));
  return date.toISOString().slice(0, 10);
}

/**
 * O status que a pessoa vê: pendente com vencimento no passado é vencido.
 * Espelha `derivePayableStatus`, com o "hoje" vindo de fora.
 */
export function effectiveStatus(status: PayableStatus, dueDate: string, today: string): PayableStatus {
  if (status === "paid") return "paid";
  return dueDate < today ? "overdue" : status;
}

/** Hoje no fuso do dono. A Vercel roda em UTC; o dono vive em São Paulo. */
export function todayInSaoPaulo(now = new Date()) {
  return new Intl.DateTimeFormat("en-CA", {
    timeZone: "America/Sao_Paulo",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(now);
}
