import Link from "next/link";
import { ArrowRight } from "lucide-react";
import { markInvoiceAsPending } from "@/app/actions/invoices";
import { setCardActive, updateCard } from "@/app/actions/cards";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { EditDisclosure } from "@/components/ui/edit-disclosure";
import { FormSubmitButton } from "@/components/form-submit-button";
import { MarkPaidButton } from "@/components/payable/mark-paid-button";
import { StatusBadge } from "@/components/status-badge";
import { ToastForm } from "@/components/toast-form";
import { ValidatedForm, ValidatedInput, ValidatedSelect } from "@/components/ui/validated-form";
import { CardBrandMark } from "@/components/cards/card-brand-mark";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { formatCurrency } from "@/lib/formatters/currency";
import { formatShortDate } from "@/lib/formatters/date";

type ManagedCard = {
  id: string;
  name: string;
  cardType: "personal" | "business";
  dueDay: number;
  isActive: boolean;
};

type CardInvoice = {
  id: string;
  cardId: string;
  amountCents: number;
  dueDate: string;
  status: "pending" | "paid" | "overdue";
};

const cardTypeLabel = {
  personal: "Pessoal",
  business: "PJ",
};

export function CardManager({ cards, invoices = [] }: { cards: ManagedCard[]; invoices?: CardInvoice[] }) {
  const invoiceByCardId = new Map(invoices.map((invoice) => [invoice.cardId, invoice]));

  return (
    <DashboardCard
      description="Cada cartão mostra a fatura do mês selecionado, o vencimento e o histórico."
      title="Cartões e faturas"
    >
      <div className="grid gap-3 md:grid-cols-2">
        {cards.length === 0 ? (
          <InlineEmpty>
            Nenhum cartão cadastrado. Adicione um cartão para começar a controlar faturas.
          </InlineEmpty>
        ) : (
          cards.map((card) => {
            const invoice = invoiceByCardId.get(card.id);

            return (
              <div
                className="rounded-lg border border-border-subtle bg-background-elevated p-4"
                key={card.id}
              >
                <div className="flex items-start justify-between gap-3">
                  <div className="flex min-w-0 items-start gap-3">
                    <CardBrandMark name={card.name} />
                    <div className="min-w-0">
                      <div className="flex flex-wrap items-center gap-2">
                        <p className="truncate font-semibold text-text-primary">{card.name}</p>
                        {card.cardType === "business" ? (
                          <span className="rounded-sm border border-border-subtle px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-[0.12em] text-text-muted">
                            PJ
                          </span>
                        ) : null}
                        {!card.isActive ? (
                          <span className="rounded-sm border border-border-subtle px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-[0.12em] text-text-muted">
                            Inativo
                          </span>
                        ) : null}
                      </div>
                      <p className="mt-1 text-sm text-text-muted">
                        {cardTypeLabel[card.cardType]} · vence dia {card.dueDay}
                      </p>
                    </div>
                  </div>
                  {invoice ? <StatusBadge status={invoice.status} /> : null}
                </div>

                <div className="mt-4 border-t border-border-subtle pt-4">
                  {invoice ? (
                    <div className="space-y-3">
                      <div className="flex items-end justify-between gap-3">
                        <div>
                          <p className="text-xs font-semibold uppercase tracking-[0.12em] text-text-muted">
                            Fatura atual
                          </p>
                          <p className="num mt-1 text-xl font-semibold text-text-primary">
                            {formatCurrency(invoice.amountCents)}
                          </p>
                        </div>
                        <p className="shrink-0 text-right text-sm text-text-muted">
                          Vence {formatShortDate(invoice.dueDate)}
                        </p>
                      </div>
                      <div className="grid gap-2 sm:grid-cols-[1fr_auto] sm:items-center">
                        <Link
                          className="focus-ring inline-flex min-h-10 items-center justify-center gap-1.5 rounded-md border border-border-default bg-background-card px-3 text-xs font-semibold text-text-secondary transition duration-200 hover:border-border-strong hover:bg-background-hover hover:text-text-primary sm:justify-start"
                          href={`/app/cards/${card.id}`}
                        >
                          Ver histórico
                          <ArrowRight size={14} aria-hidden="true" />
                        </Link>
                        {invoice.status !== "paid" ? (
                          <MarkPaidButton payableId={invoice.id} payableType="invoice" variant="secondary">
                            Marcar paga
                          </MarkPaidButton>
                        ) : (
                          <ToastForm action={markInvoiceAsPending} successMessage="Fatura reaberta.">
                            <input name="invoiceId" type="hidden" value={invoice.id} />
                            <FormSubmitButton pendingLabel="Reabrindo..." variant="secondary">
                              Reabrir
                            </FormSubmitButton>
                          </ToastForm>
                        )}
                      </div>
                    </div>
                  ) : (
                    <div className="grid gap-3 sm:grid-cols-[1fr_auto] sm:items-center">
                      <p className="text-sm text-text-muted">Nenhuma fatura no mês selecionado.</p>
                      <Link
                        className="focus-ring inline-flex min-h-10 items-center justify-center gap-1.5 rounded-md border border-border-default bg-background-card px-3 text-xs font-semibold text-text-secondary transition duration-200 hover:border-border-strong hover:bg-background-hover hover:text-text-primary sm:justify-start"
                        href={`/app/cards/${card.id}`}
                      >
                        Ver histórico
                        <ArrowRight size={14} aria-hidden="true" />
                      </Link>
                    </div>
                  )}
                </div>

                <div className="mt-4 flex justify-stretch sm:justify-end">
                  <ToastForm
                    className="w-full sm:w-auto"
                    action={setCardActive}
                    successMessage={card.isActive ? "Cartão inativado." : "Cartão reativado."}
                  >
                    <input name="cardId" type="hidden" value={card.id} />
                    <input name="isActive" type="hidden" value={card.isActive ? "false" : "true"} />
                    <FormSubmitButton
                      pendingLabel={card.isActive ? "Inativando..." : "Reativando..."}
                      variant="secondary"
                    >
                      {card.isActive ? "Inativar" : "Reativar"}
                    </FormSubmitButton>
                  </ToastForm>
                </div>

                <EditDisclosure className="mt-4">
                  <ValidatedForm action={updateCard} successMessage="Cartão atualizado." className="grid gap-3">
                    <input name="cardId" type="hidden" value={card.id} />
                    <ValidatedInput
                      className="field-input"
                      defaultValue={card.name}
                      name="name"
                      required
                    />
                    <div className="grid gap-3 sm:grid-cols-2">
                      <ValidatedSelect
                        className="field-input"
                        defaultValue={card.cardType}
                        name="cardType"
                      >
                        <option value="personal">Pessoal</option>
                        <option value="business">PJ</option>
                      </ValidatedSelect>
                      <ValidatedInput
                        className="field-input"
                        defaultValue={card.dueDay}
                        inputMode="numeric"
                        max={31}
                        min={1}
                        name="dueDay"
                        required
                        type="number"
                      />
                    </div>
                    <FormSubmitButton pendingLabel="Salvando...">Salvar cartão</FormSubmitButton>
                  </ValidatedForm>
                </EditDisclosure>
              </div>
            );
          })
        )}
      </div>
    </DashboardCard>
  );
}
