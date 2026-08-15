import { cookies } from "next/headers";
import {
  getCurrentMonthForUser,
  getCurrentMonthParts,
  getMonthByParts,
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

/**
 * Builds the switcher list: every existing month plus the current calendar
 * month (so the user can always navigate back to "now"), newest first.
 */
export async function getMonthSwitcherData(userId: string) {
  const rows = await getMonthsForUser(userId);
  const current = getCurrentMonthParts();
  const items = rows.map((row) => ({ month: row.month, year: row.year }));

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
