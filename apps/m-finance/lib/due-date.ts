/**
 * Builds an ISO `yyyy-mm-dd` due date inside a given financial month from a
 * day-of-month, clamping to the last valid day (e.g. day 31 in February).
 * The financial month is always the current month, so a full date picker is
 * unnecessary friction — the user only ever picks a day.
 */
export function composeMonthDate(year: number, month: number, day: number) {
  const lastDay = new Date(year, month, 0).getDate();
  const clamped = Math.min(Math.max(Math.round(day), 1), lastDay);
  return `${year}-${String(month).padStart(2, "0")}-${String(clamped).padStart(2, "0")}`;
}

/** Parses a day-of-month from form input. Empty or invalid returns undefined. */
export function parseDueDay(value: FormDataEntryValue | null) {
  const raw = String(value ?? "").trim();
  if (!raw) return undefined;
  const day = Number(raw);
  return Number.isFinite(day) && day >= 1 && day <= 31 ? Math.round(day) : undefined;
}

/** Reads the day-of-month from an ISO date string for prefilling day inputs. */
export function dayFromIsoDate(date: string) {
  return new Date(`${date}T12:00:00`).getDate();
}
