"use client";

import { InlineEmpty } from "@/components/ui/inline-empty";
import { toDueDateBuckets } from "@/lib/calculations/charts/due-dates";
import { formatCurrency } from "@/lib/formatters/currency";

/**
 * A pressão do mês, dia a dia.
 *
 * Sem recharts: são retângulos numa grade, e trazer um motor de eixos para
 * desenhar retângulos custa mais do que entrega. A intensidade percorre a
 * opacidade do sódio — uma rampa, como manda `lib/ui/colors.ts`, e não uma
 * escala de matizes.
 */
export function DueDateHeatmap({
  items,
  month,
  year,
}: {
  items: { dueDate: string; amountCents: number }[];
  month: number;
  year: number;
}) {
  const buckets = toDueDateBuckets(items, year, month);
  const total = buckets.reduce((sum, bucket) => sum + bucket.cents, 0);

  if (total === 0) {
    return <InlineEmpty>Nenhum vencimento neste mês.</InlineEmpty>;
  }

  const heaviest = buckets.reduce((worst, bucket) =>
    bucket.cents > worst.cents ? bucket : worst,
  );

  return (
    <div>
      <ol className="flex flex-wrap gap-1">
        {buckets.map((bucket) => (
          <li key={bucket.day}>
            <div
              className="flex h-8 w-8 items-center justify-center rounded-sm border border-border-subtle text-[10px] text-text-muted"
              // Opacidade e não cor: a rampa é de clareza, e o piso de 8%
              // mantém o dia com vencimento leve visível como preenchido.
              style={{
                backgroundColor: `color-mix(in srgb, var(--signal-fill) ${
                  bucket.cents === 0 ? 0 : 8 + bucket.intensity * 72
                }%, transparent)`,
              }}
              title={`Dia ${bucket.day}: ${formatCurrency(bucket.cents)}`}
            >
              {bucket.day}
            </div>
          </li>
        ))}
      </ol>
      <p className="mt-3 text-xs text-text-muted">
        Dia mais pesado: <span className="num text-text-secondary">{heaviest.day}</span>, com{" "}
        <span className="num text-text-secondary">{formatCurrency(heaviest.cents)}</span> vencendo.
      </p>
    </div>
  );
}
