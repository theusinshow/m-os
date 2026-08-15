import { createInvoice } from "@/app/actions/invoices";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { FormSubmitButton } from "@/components/form-submit-button";
import { ValidatedForm, ValidatedInput, ValidatedSelect } from "@/components/ui/validated-form";

type Card = {
  id: string;
  name: string;
  cardType: "personal" | "business";
};

export function InvoiceFormCard({ cards }: { cards: Card[] }) {
  return (
    <DashboardCard
      description="Lance só o total do mês. O vencimento usa o dia do próprio cartão."
      title="Adicionar fatura"
    >
      <ValidatedForm
        action={createInvoice}
        successMessage="Fatura adicionada."
        resetOnSuccess
        className="grid gap-4 md:grid-cols-[1fr_200px_auto] md:items-end"
      >
        <div>
          <label className="mb-2 block text-sm font-medium text-text-secondary" htmlFor="invoice-card">
            Cartão
          </label>
          <ValidatedSelect
            className="focus-ring min-h-11 w-full rounded-md border border-border-subtle bg-background-elevated px-3 text-sm text-text-primary"
            id="invoice-card"
            name="cardId"
            required
          >
            {cards.map((card) => (
              <option key={card.id} value={card.id}>
                {card.name}
                {card.cardType === "business" ? " PJ" : ""}
              </option>
            ))}
          </ValidatedSelect>
        </div>
        <div>
          <label className="mb-2 block text-sm font-medium text-text-secondary" htmlFor="invoice-amount">
            Valor da fatura
          </label>
          <ValidatedInput
            className="focus-ring min-h-11 w-full rounded-md border border-border-subtle bg-background-elevated px-3 text-sm text-text-primary placeholder:text-text-muted"
            id="invoice-amount"
            inputMode="decimal"
            name="amount"
            placeholder="900,00"
            required
          />
        </div>
        <FormSubmitButton pendingLabel="Salvando...">Salvar fatura</FormSubmitButton>
      </ValidatedForm>
    </DashboardCard>
  );
}
