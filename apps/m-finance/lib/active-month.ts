import { cookies } from "next/headers";
import {
  getCurrentMonthForUser,
  getCurrentMonthParts,
  getMonthByParts,
  getMonthIdsWithActivity,
  getMonthsForUser,
} from "@/lib/months";
import { formatMonthLabel } from "@/lib/formatters/date";

export const ACTIVE_MONTH_COOKIE = "mf-active-month";

export function monthValue(month: number, year: number) {
  return `${year}-${String(month).padStart(2, "0")}`;
}

export function parseMonthValue(value: string | undefined | null) {
  if (!value) return null;
  const match = /^(\d{4})-(\d{2})$/.exec(value);
  if (!match) return null;
  const year = Number(match[1]);
  const month = Number(match[2]);
  if (month < 1 || month > 12 || year < 2020) return null;
  return { month, year };
}

/**
 * The month the user is currently viewing. Defaults to the real calendar month
 * when the cookie is unset or malformed, so the app behaves as before until the
 * user picks another month from the switcher.
 */
export async function getActiveMonthParts() {
  const store = await cookies();
  return parseMonthValue(store.get(ACTIVE_MONTH_COOKIE)?.value) ?? getCurrentMonthParts();
}

export async function isViewingCurrentMonth() {
  const active = await getActiveMonthParts();
  const current = getCurrentMonthParts();
  return active.month === current.month && active.year === current.year;
}

/**
 * Resolves the active month to a real record. Falls back to the current month
 * record (which may be null when it hasn't been created yet) if the selected
 * month no longer exists.
 */
export async function getActiveMonthForUser(userId: string) {
  const { month, year } = await getActiveMonthParts();
  const record = await getMonthByParts(userId, month, year);
  if (record) return record;
  return getCurrentMonthForUser(userId);
}

type MonthParts = { month: number; year: number };

function monthKey({ month, year }: MonthParts) {
  return `${year}-${String(month).padStart(2, "0")}`;
}

/**
 * O que o seletor de mês deve oferecer.
 *
 * Uma linha em `months` nasce por muito motivo que não é o dono ter usado o
 * mês: uma série parcelada de 22 parcelas cria 22 delas de uma vez. O passado
 * vazio não tem nada para mostrar — e era exatamente onde o app reabria,
 * exibindo R$ 0,00 como se fosse a vida real. O futuro vazio fica: é onde o
 * mês novo nasce.
 */
export function visibleSwitcherMonths(
  items: MonthParts[],
  keysWithActivity: Set<string>,
  current: MonthParts,
): MonthParts[] {
  return items.filter((item) => {
    const isPast = item.year < current.year || (item.year === current.year && item.month < current.month);
    if (!isPast) return true;
    return keysWithActivity.has(monthKey(item));
  });
}

/**
 * Builds the switcher list: every month that has something to show plus the
 * current calendar month (so the user can always navigate back to "now"),
 * newest first.
 */
export async function getMonthSwitcherData(userId: string) {
  const rows = await getMonthsForUser(userId);
  const current = getCurrentMonthParts();
  const activity = await getMonthIdsWithActivity(userId);
  const keysWithActivity = new Set(
    rows.filter((row) => activity.has(row.id)).map((row) => monthKey(row)),
  );
  const items = visibleSwitcherMonths(
    rows.map((row) => ({ month: row.month, year: row.year })),
    keysWithActivity,
    current,
  );

  if (!items.some((item) => item.month === current.month && item.year === current.year)) {
    items.push(current);
  }

  items.sort((a, b) => b.year - a.year || b.month - a.month);

  const options = items.map((item) => ({
    value: monthValue(item.month, item.year),
    label: formatMonthLabel(new Date(item.year, item.month - 1, 1)),
    isCurrent: item.month === current.month && item.year === current.year,
  }));

  const active = await getActiveMonthParts();
  return { options, activeValue: monthValue(active.month, active.year) };
}
