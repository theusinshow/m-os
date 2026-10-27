import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { formatCurrency } from "@/lib/formatters/currency";
import type { CardMonthLine } from "@/lib/calculations/forecast";

/**
 * O total dos cartões no mês, com o que é certo separado do que é palpite.
 *
 * Antes a página só listava cartão por cartão, e um mês sem fatura lançada
 * aparecia como "nenhuma fatura" — que qualquer um lê como "nada a pagar".
 */
export function CardMonthSummary({
  lines,
  monthLabel,
}: {
  lines: CardMonthLine[];
  monthLabel: string;
}) {
  const totalCents = lines.reduce((sum, line) => sum + line.amountCents, 0);
  const confirmedCents = lines
    .filter((line) => line.source === "actual")
    .reduce((sum, line) => sum + line.amountCents, 0);
  const estimated = lines.filter((line) => line.source === "estimated");
  const estimatedCents = estimated.reduce((sum, line) => sum + line.amountCents, 0);
  const paidCents = lines
    .filter((line) => line.invoice?.status === "paid")
    .reduce((sum, line) => sum + line.amountCents, 0);
  const toPayCents = totalCents - paidCents;
  const missing = lines.filter((line) => line.source === "none").length;

  return (
    <DashboardCard accent>
      <p className="text-sm text-text-muted">Cartões em {monthLabel}</p>
      <p className="num mt-2 text-4xl font-semibold text-text-primary">
        {estimatedCents > 0 ? "≈ " : ""}
        {formatCurrency(toPayCents)}
      </p>
      <p className="mt-1.5 text-sm text-text-muted">
        {toPayCents === 0 && totalCents > 0 ? "Tudo pago neste mês." : "falta pagar"}
      </p>

      <dl className="mt-5 grid grid-cols-2 gap-3 border-t border-border-subtle pt-4 text-sm sm:grid-cols-4">
        <div>
          <dt className="text-xs uppercase tracking-[0.12em] text-text-muted">Total do mês</dt>
          <dd className="num mt-1 font-semibold text-text-primary">{formatCurrency(totalCents)}</dd>
        </div>
        <div>
          <dt className="text-xs uppercase tracking-[0.12em] text-text-muted">Confirmado</dt>
          <dd className="num mt-1 font-semibold text-text-primary">
            {formatCurrency(confirmedCents)}
          </dd>
        </div>
        <div>
          <dt className="text-xs uppercase tracking-[0.12em] text-text-muted">Estimado</dt>
          <dd className="num mt-1 font-semibold text-text-secondary">
            {formatCurrency(estimatedCents)}
          </dd>
        </div>
        <div>
          <dt className="text-xs uppercase tracking-[0.12em] text-text-muted">Pago</dt>
          <dd className="num mt-1 font-semibold text-status-positive">{formatCurrency(paidCents)}</dd>
        </div>
      </dl>

      {estimated.length > 0 ? (
        <p className="mt-4 text-sm leading-6 text-text-muted">
          {estimated.length === 1
            ? "1 fatura ainda é estimada"
            : `${estimated.length} faturas ainda são estimadas`}{" "}
          pela média das últimas faturas de cada cartão. Quando o banco fechar a fatura, confirme o
          valor real no cartão.
        </p>
      ) : null}
      {missing > 0 ? (
        <p className="mt-2 text-sm leading-6 text-text-muted">
          {missing === 1 ? "1 cartão ativo" : `${missing} cartões ativos`} sem fatura e sem
          histórico para estimar.
        </p>
      ) : null}
    </DashboardCard>
  );
}
