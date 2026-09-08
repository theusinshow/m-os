import { formatCurrency } from "@/lib/formatters/currency";
import { splitPersonalBusiness } from "@/lib/calculations/commitments";

type Invoice = {
  id: string;
  name: string;
  amountCents: number;
  cardType: "personal" | "business";
};

/**
 * Quanto das faturas do mês é da empresa.
 *
 * O card PJ entrava no mesmo total do gasto pessoal. Um MEI que paga a própria
 * fatura pelo CNPJ estava lendo um mês R$ 1.300 mais apertado do que o dele.
 */
export function PersonalBusinessCard({ invoices }: { invoices: Invoice[] }) {
  const split = splitPersonalBusiness(invoices);

  if (split.businessCents === 0) {
    return null;
  }

  const businessInvoices = invoices.filter((invoice) => invoice.cardType === "business");

  return (
    <div className="rounded-xl border border-border-subtle bg-background-card/95 p-4 shadow-lg shadow-black/10 sm:p-5">
      <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
        <h2 className="text-sm font-semibold uppercase tracking-[0.16em] text-text-muted">
          Pessoal × PJ
        </h2>
        <p className="text-xs text-text-muted">Faturas do mês</p>
      </div>

      <div
        aria-hidden="true"
        className="mt-4 flex h-2 overflow-hidden rounded-full bg-background-elevated"
      >
        <span
          className="bg-text-secondary/70"
          style={{ width: `${100 - split.businessPercent}%` }}
        />
        <span className="bg-accent/70" style={{ width: `${split.businessPercent}%` }} />
      </div>

      <div className="mt-4 grid gap-3 sm:grid-cols-2">
        <div>
          <p className="text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">
            Pessoal
          </p>
          <p className="num mt-1 text-xl font-semibold text-text-primary">
            {formatCurrency(split.personalCents)}
          </p>
        </div>
        <div>
          <p className="text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">
            PJ · {split.businessPercent}%
          </p>
          <p className="num mt-1 text-xl font-semibold text-accent">
            {formatCurrency(split.businessCents)}
          </p>
          <p className="mt-1 text-xs text-text-muted">
            {businessInvoices.map((invoice) => invoice.name).join(", ")}
          </p>
        </div>
      </div>
    </div>
  );
}
