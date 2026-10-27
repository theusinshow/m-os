import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { formatCurrency } from "@/lib/formatters/currency";
import type { CardMonthLine, ForecastCard } from "@/lib/calculations/forecast";

type Column = { key: string; label: string; lines: CardMonthLine[] };

function cell(line: CardMonthLine | undefined) {
  if (!line || line.source === "none") return <span className="text-text-muted">—</span>;
  if (line.source === "estimated") {
    return <span className="text-text-muted">≈ {formatCurrency(line.amountCents)}</span>;
  }
  return <span className="text-text-primary">{formatCurrency(line.amountCents)}</span>;
}

/**
 * Os próximos meses de cada cartão numa grade: o que já está lançado aparece
 * cheio, o que é estimado aparece com "≈". É a resposta para "quanto vou pagar
 * de cartão em novembro" sem trocar de mês.
 */
export function CardForecastGrid({ cards, columns }: { cards: ForecastCard[]; columns: Column[] }) {
  if (cards.length === 0 || columns.length === 0) return null;

  return (
    <DashboardCard
      description="Lançado aparece cheio; estimado aparece com ≈ até a fatura fechar."
      title="Próximos meses"
    >
      <div className="-mx-1 overflow-x-auto px-1">
        <table className="w-full min-w-[32rem] text-sm">
          <thead>
            <tr className="text-left text-xs uppercase tracking-[0.12em] text-text-muted">
              <th className="pb-2 pr-3 font-semibold" scope="col">
                Cartão
              </th>
              {columns.map((column) => (
                <th className="pb-2 pl-3 text-right font-semibold" key={column.key} scope="col">
                  {column.label}
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="num">
            {cards.map((card) => (
              <tr className="border-t border-border-subtle" key={card.id}>
                <th className="py-2.5 pr-3 text-left font-medium text-text-secondary" scope="row">
                  {card.name}
                  {card.cardType === "business" ? " PJ" : ""}
                </th>
                {columns.map((column) => (
                  <td className="py-2.5 pl-3 text-right" key={column.key}>
                    {cell(column.lines.find((line) => line.card.id === card.id))}
                  </td>
                ))}
              </tr>
            ))}
            <tr className="border-t border-border-strong">
              <th className="pt-2.5 pr-3 text-left font-semibold text-text-primary" scope="row">
                Total
              </th>
              {columns.map((column) => {
                const total = column.lines.reduce((sum, line) => sum + line.amountCents, 0);
                const hasEstimate = column.lines.some((line) => line.source === "estimated");
                return (
                  <td className="pt-2.5 pl-3 text-right font-semibold text-text-primary" key={column.key}>
                    {hasEstimate ? "≈ " : ""}
                    {formatCurrency(total)}
                  </td>
                );
              })}
            </tr>
          </tbody>
        </table>
      </div>
    </DashboardCard>
  );
}
