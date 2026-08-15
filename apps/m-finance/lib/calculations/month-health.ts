import type { MonthHealth } from "@/db/schema";

type MonthHealthInput = {
  estimatedRemainingCents: number;
  overdueCents: number;
  dueSoonCount: number;
};

export function classifyMonthHealth({
  estimatedRemainingCents,
  overdueCents,
  dueSoonCount,
}: MonthHealthInput): MonthHealth {
  if (estimatedRemainingCents < 0 || overdueCents > 0) {
    return "negative";
  }

  if (estimatedRemainingCents <= 30000 || dueSoonCount >= 5) {
    return "tight";
  }

  if (estimatedRemainingCents <= 100000 || dueSoonCount >= 3) {
    return "fair";
  }

  return "positive";
}
