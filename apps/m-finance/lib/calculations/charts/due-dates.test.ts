import { describe, expect, it } from "vitest";
import { toDueDateBuckets } from "@/lib/calculations/charts/due-dates";

describe("toDueDateBuckets", () => {
  it("devolve uma celula por dia do mes", () => {
    expect(toDueDateBuckets([], 2026, 4)).toHaveLength(30);
    expect(toDueDateBuckets([], 2026, 8)).toHaveLength(31);
  });

  it("conhece fevereiro comum e bissexto", () => {
    expect(toDueDateBuckets([], 2026, 2)).toHaveLength(28);
    expect(toDueDateBuckets([], 2024, 2)).toHaveLength(29);
  });

  it("soma os vencimentos do mesmo dia", () => {
    const buckets = toDueDateBuckets(
      [
        { dueDate: "2026-08-10", amountCents: 30_000 },
        { dueDate: "2026-08-10", amountCents: 20_000 },
        { dueDate: "2026-08-05", amountCents: 10_000 },
      ],
      2026,
      8,
    );

    expect(buckets[9].cents).toBe(50_000);
    expect(buckets[4].cents).toBe(10_000);
  });

  it("deixa em zero o dia sem vencimento", () => {
    const buckets = toDueDateBuckets([{ dueDate: "2026-08-10", amountCents: 30_000 }], 2026, 8);

    expect(buckets[0].cents).toBe(0);
    expect(buckets[0].intensity).toBe(0);
  });

  it("normaliza a intensidade pelo dia mais pesado", () => {
    const buckets = toDueDateBuckets(
      [
        { dueDate: "2026-08-10", amountCents: 100_000 },
        { dueDate: "2026-08-20", amountCents: 50_000 },
      ],
      2026,
      8,
    );

    expect(buckets[9].intensity).toBe(1);
    expect(buckets[19].intensity).toBe(0.5);
  });

  it("ignora vencimento de outro mes", () => {
    const buckets = toDueDateBuckets(
      [
        { dueDate: "2026-09-10", amountCents: 100_000 },
        { dueDate: "2026-08-10", amountCents: 40_000 },
      ],
      2026,
      8,
    );

    expect(buckets[9].cents).toBe(40_000);
    expect(buckets.reduce((total, bucket) => total + bucket.cents, 0)).toBe(40_000);
  });

  it("nao quebra num mes sem nenhum vencimento", () => {
    const buckets = toDueDateBuckets([], 2026, 8);

    expect(buckets.every((bucket) => bucket.intensity === 0)).toBe(true);
  });
});
